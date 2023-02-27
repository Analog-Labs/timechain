#![allow(clippy::type_complexity)]
use crate::{
	communication::validator::{topic, GossipValidator},
	inherents::update_shared_group_key,
	kv::TimeKeyvault,
	tss_event_handler_helper::{aggregator_event_sign, handler_partial_signature_generate_req},
	Client, WorkerParams, TW_LOG,
};
use borsh::{BorshDeserialize, BorshSerialize};
use codec::{Decode, Encode};
use futures::{channel::mpsc::Receiver as FutReceiver, FutureExt, StreamExt};
use log::{debug, error, info, warn};
use sc_client_api::{
	Backend, FinalityNotification, FinalityNotifications, StorageEventStream, StorageKey,
	StorageNotification,
};
use sc_network_gossip::GossipEngine;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_consensus::SyncOracle;
use sp_runtime::{
	generic::BlockId,
	traits::{Block, Header},
};
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
	time::Duration,
};
use time_primitives::{
	crypto::Public,
	sharding::{Shard, FILTER_PALLET_KEY_BYTES},
	TimeApi, TimeId, KEY_TYPE,
};
use tokio::sync::Mutex as TokioMutex;
use tss::{
	frost_dalek::{compute_message_hash, signature::ThresholdSignature, SignatureAggregator},
	local_state_struct::TSSLocalStateData,
	tss_event_model::{PartialMessageSign, TSSData, TSSEventType},
	utils::{get_receive_params_msg, make_gossip_tss_data},
};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct TimeWorker<B: Block, C, R, BE, SO> {
	pub(crate) client: Arc<C>,
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	finality_notifications: FinalityNotifications<B>,
	shard_storage_notifications: StorageEventStream<B::Hash>,
	gossip_engine: GossipEngine<B>,
	gossip_validator: Arc<GossipValidator<B>>,
	sign_data_receiver: Arc<TokioMutex<FutReceiver<(u64, Vec<u8>)>>>,
	pub(crate) kv: TimeKeyvault,
	sync_oracle: SO,
	pub(crate) tss_local_states: HashMap<u64, TSSLocalStateData>,
	known_sets: HashSet<u64>,
}

impl<B, C, R, BE, SO> TimeWorker<B, C, R, BE, SO>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B>,
	SO: SyncOracle + Send + Sync + Clone + 'static,
{
	pub(crate) fn new(worker_params: WorkerParams<B, C, R, BE, SO>) -> Self {
		let WorkerParams {
			client,
			backend,
			runtime,
			gossip_engine,
			gossip_validator,
			sync_oracle,
			kv,
			sign_data_receiver,
		} = worker_params;

		// TODO: threshold calc if required
		TimeWorker {
			finality_notifications: client.finality_notification_stream(),
			// TODO: handle this unwrap
			shard_storage_notifications: client
				.storage_changes_notification_stream(
					Some(&[StorageKey(FILTER_PALLET_KEY_BYTES.to_vec())]),
					None,
				)
				.unwrap(),
			client,
			backend,
			runtime,
			gossip_engine,
			gossip_validator,
			sync_oracle,
			kv,
			tss_local_states: HashMap::new(),
			sign_data_receiver,
			known_sets: HashSet::new(),
		}
	}

	// helper to create new state for new id
	pub(crate) fn create_tss_state(&self) -> TSSLocalStateData {
		let mut tss_local_state = TSSLocalStateData::new();
		tss_local_state.key_type = Some(KEY_TYPE);
		tss_local_state.keystore = self.kv.get_store();
		tss_local_state
	}

	fn on_storage_update(&mut self, notification: StorageNotification<B::Hash>) {
		// check if any new shards are registered
		debug!(target: TW_LOG, "Pallet storage udate received.");
		let keys = self.kv.public_keys();
		if keys.len() != 1 {
			error!(target: TW_LOG, "Expected exactly one Time key - can not participate.");
			return;
		}
		let our_key = time_primitives::TimeId::decode(&mut keys[0].as_ref()).unwrap();
		let in_block = BlockId::Hash(notification.block);
		// TODO: stabilize unwrap
		let shards = self.runtime.runtime_api().get_shards(&in_block).unwrap();
		// check if we're member in new shards
		let new_shards: Vec<_> =
			shards.into_iter().filter(|(id, _)| self.known_sets.contains(id)).collect();
		for (id, new_shard) in new_shards {
			if new_shard.members().contains(&our_key) {
				// participate in keygen for new shard
				info!(target: TW_LOG, "Participating in new keygen for id {}", id);
			} else {
				debug!(target: TW_LOG, "Not a member of shard {}", id);
			}
			self.known_sets.insert(id);
		}
	}

	// checks if we're present in specific shard
	// at latste finalized block
	fn is_member_of_shard(&self, shart_id: u64) -> bool {
		let at = self.backend.blockchain().last_finalized().unwrap();
		if let Some(id) = self.account_id() {
			if let Ok(Some(members)) =
				self.runtime.runtime_api().get_shard_members(&BlockId::Hash(at), shart_id)
			{
				members.iter().any(|s| s == &id)
			} else {
				false
			}
		} else {
			false
		}
	}

	fn new_keygen_for_new_id(&mut self, shard_id: u64, new_shard: Shard) {
		let mut state = self.create_tss_state();
		let keys = self.kv.public_keys();
		if keys.is_empty() {
			warn!(target: TW_LOG, "No time key found, please inject one.");
			return;
		}
		let id = keys[0].clone();
		// starting TSS initialization if not yet done

		if state.local_peer_id.is_none() {
			state.local_peer_id = Some(id.to_string());
			// TODO: reset if not all comply
			// slashing goes below too?
			/*			let msg = make_gossip_tss_data(
							id.to_string(),
							get_reset_tss_msg("Reinit state".to_string()).unwrap_or_default(),
							TSSEventType::ReceiveParams,
						)
						.unwrap();
						self.send(msg);
			*/
			// initialize TSS
			// TODO: assign collector role to at most one node for protocol to work
			//let collector_id_bytes =
			//	hex::decode("48640c12bc1b351cf4b051ac1cf7b5740765d02e34989d0a9dd935ce054ebb21")
			//		.unwrap();
			//let collector_id = sp_application_crypto::sr25519::Public::from_raw(
			//	array_bytes::vec2array(collector_id_bytes).unwrap(),
			//);
			//#[allow(unused_mut)]
			//let mut for_tests = false;
			//// we allow alice to be collector and aggregator in tests
			//#[cfg(test)]
			//if id == crate::tests::kv_tests::Keyring::Alice.public() {
			//	for_tests = true;
			//}
			let collector_id = new_shard.collector();
			if id == Public::decode(&mut &collector_id.encode()[..]).unwrap() {
				state.is_node_collector = true;
				// TODO: assign aggregator role to at most one node for protocol to work
				state.is_node_aggregator = true;
				if let Ok(peer_id_data) = get_receive_params_msg(id.to_string(), state.tss_params) {
					if let Ok(data) = make_gossip_tss_data(
						id.to_string(),
						peer_id_data,
						TSSEventType::ReceiveParams(shard_id),
					) {
						self.send(data);
						info!(target: TW_LOG, "TSS keygen initiated by {id}");
					} else {
						error!(target: TW_LOG, "Failed to prepare initial TSS message");
					}
				} else {
					error!(target: TW_LOG, "Unable to make publish peer id msg");
				}
			}
			self.tss_local_states.insert(shard_id, state);
		}
	}

	/// On each grandpa finality we're initiating gossip to all other authorities to acknowledge
	fn on_finality(&mut self, notification: FinalityNotification<B>) {
		info!(target: TW_LOG, "Got new finality notification: {}", notification.header.number());
	}

	fn process_sign_message(&mut self, shard_id: u64, data: Vec<u8>) {
		// do sig
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			let context = state.context;
			let msg_hash = compute_message_hash(&context, &data);
			//add node in msg_pool
			if state.msg_pool.get(&msg_hash).is_none() {
				state.msg_pool.insert(msg_hash, data.clone());
				//process msg if req already received
				if let Some(pending_msg_req) = state.msgs_signature_pending.get(&msg_hash) {
					let request = PartialMessageSign {
						msg_hash,
						signers: pending_msg_req.to_vec(),
					};
					let encoded = request.try_to_vec().unwrap();
					if let Some((peer_id, data, message_type)) =
						handler_partial_signature_generate_req(state, &encoded)
					{
						if let Ok(encoded_data) = data.try_to_vec() {
							if let Ok(data) =
								make_gossip_tss_data(peer_id, encoded_data, message_type)
							{
								self.gossip_engine.gossip_message(topic::<B>(), data, false);
							} else {
								log::error!(
									"TSS::error making gossip data for encoded participant"
								);
							}
						} else {
							//log error
							log::error!("TSS::tss error");
						}
					}
					state.msgs_signature_pending.remove(&msg_hash);
				} else {
					log::debug!(
						target: TW_LOG,
						"New data for signing received: {:?} with hash {:?}",
						data,
						msg_hash
					);
				}
			} else {
				log::warn!(
					target: TW_LOG,
					"Message with hash {} is already in process of signing",
					hex::encode(msg_hash)
				);
			}
			//creating signature aggregator for msg
			if state.is_node_aggregator {
				//all nodes should share the same message hash
				//to verify the threshold signature
				let mut aggregator = SignatureAggregator::new(
					state.tss_params,
					state.local_finished_state.clone().unwrap().0,
					&context,
					&data,
				);

				for com in state.others_commitment_share.clone() {
					aggregator.include_signer(
						com.public_commitment_share_list.participant_index,
						com.public_commitment_share_list.commitments[0],
						com.public_key,
					);
				}

				//including aggregator as a signer
				aggregator.include_signer(
					state.local_index.unwrap(),
					state.local_commitment_share.clone().unwrap().0.commitments[0],
					state.local_public_key.clone().unwrap(),
				);

				//this signers list will be used by other nodes to verify themselves.
				let signers = aggregator.get_signers();
				state.current_signers = signers.clone();

				// //sign msg from aggregator side
				aggregator_event_sign(state, msg_hash);

				let sign_msg_req = PartialMessageSign {
					msg_hash,
					signers: signers.clone(),
				};

				if let Ok(data) = make_gossip_tss_data(
					state.local_peer_id.clone().unwrap_or_default(),
					sign_msg_req.try_to_vec().unwrap(),
					TSSEventType::PartialSignatureGenerateReq(shard_id),
				) {
					self.send(data);
					info!(target: TW_LOG, "TSS peer collection req sent");
				} else {
					error!(
						target: TW_LOG,
						"Failed to pack TSS message for signing hash: {}",
						hex::encode(msg_hash)
					);
				}
			}
		} else {
			error!(target: TW_LOG, "Given shard ID is not found {shard_id}");
		}
	}

	pub(crate) fn send(&mut self, msg: Vec<u8>) {
		info!(target: TW_LOG, "Sending new gossip message");
		self.gossip_engine.gossip_message(topic::<B>(), msg, false);
	}

	// Creates opitonal ID from our Time Key
	pub(crate) fn id(&self) -> Option<String> {
		match self.kv.public_keys() {
			keys if !keys.is_empty() => Some(keys[0].to_string()),
			_ => {
				log::warn!("TSS: No local peer identity ");
				None
			},
		}
	}

	// Creates optional TimeId from set Time Key
	fn account_id(&self) -> Option<TimeId> {
		let keys = self.kv.public_keys();
		if keys.is_empty() {
			warn!(target: TW_LOG, "No time key found, please inject one.");
			None
		} else {
			let id = keys[0];
			Some(TimeId::decode(&mut id.as_ref()).unwrap())
		}
	}

	#[allow(dead_code)]
	// Using this method each validator can, and should, submit shared `GroupKey` key to runtime
	pub(crate) fn submit_key_as_inherent(&self, key: [u8; 32], set_id: u64) {
		update_shared_group_key(set_id, key);
	}

	/// On each gossip we process it, verify signature and update our tracker
	async fn on_gossip(&mut self, tss_gossiped_data: TSSData) {
		info!(target: TW_LOG, "Processing new gossip");
		match tss_gossiped_data.tss_event_type {
			//nodes will be receiving this event to make participant using params
			TSSEventType::ReceiveParams(shard_id) => {
				// check if we're part of given shard
				if self.is_member_of_shard(shard_id) {
					self.handler_receive_params(shard_id, &tss_gossiped_data.tss_data).await;
				}
			},
			// nodes will receive peer id of other nodes and will add it to their list
			TSSEventType::ReceivePeerIDForIndex(shard_id) => {
				self.handler_receive_peer_id_for_index(shard_id, &tss_gossiped_data.tss_data)
					.await;
			}, //nodes receives peers with collector participants
			TSSEventType::ReceivePeersWithColParticipant(shard_id) => {
				self.handler_receiver_peers_with_col_participant(&tss_gossiped_data.tss_data)
					.await;
			},
			//nodes will receive participant and will add will go to round one state
			TSSEventType::ReceiveParticipant(shard_id) => {
				self.handler_receive_participant(&tss_gossiped_data.tss_data).await;
			},
			//nodes will receive their secret share and take state to round two
			TSSEventType::ReceiveSecretShare(shard_id) => {
				info!(target: TW_LOG, "Received ReceiveSecretShare");
				self.handler_receive_secret_share(&tss_gossiped_data.tss_data).await;
			},

			//received commitments of other nodes who are participating in TSS process
			TSSEventType::ReceiveCommitment(shard_id) => {
				self.handler_receive_commitment(&tss_gossiped_data.tss_data).await;
			},

			//event received by collector and partial sign request is received
			TSSEventType::PartialSignatureGenerateReq(shard_id) => {
				if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
					handler_partial_signature_generate_req(state, &tss_gossiped_data.tss_data);
				}
			},

			//received partial signature. threshold signature to be made by aggregator
			TSSEventType::PartialSignatureReceived(shard_id) => {
				self.handler_partial_signature_received(&tss_gossiped_data.tss_data).await;
			},

			//verify threshold signature generated by aggregator
			TSSEventType::VerifyThresholdSignature(shard_id) => {
				self.handler_verify_threshold_signature(&tss_gossiped_data.tss_data).await;
			},

			//received resetting tss state request
			TSSEventType::ResetTSSState(shard_id) => {
				self.handler_reset_tss_state(&tss_gossiped_data.tss_data).await;
			},
		}
	}

	/// Sends signature to runtime storage through runtime API
	/// # Params
	/// * ts - ThresholdSignature to be stored
	pub(crate) fn store_signature(&mut self, ts: ThresholdSignature) {
		let key_bytes = ts.to_bytes();
		let auth_key = self.kv.public_keys()[0].clone();
		let at = self.backend.blockchain().last_finalized().unwrap();
		let signature = self.kv.sign(&auth_key, &key_bytes).unwrap();
		// FIXME: error handle
		drop(self.runtime.runtime_api().store_signature(
			&BlockId::Hash(at),
			auth_key,
			signature,
			key_bytes.to_vec(),
			// TODO: construct or receive proper id
			0.into(),
		));
	}

	/// Our main worker main process - we act on grandpa finality and gossip messages for interested
	/// topics
	pub(crate) async fn run(&mut self) {
		let mut gossips = Box::pin(
			self.gossip_engine
				.messages_for(topic::<B>())
				.filter_map(|notification| async move {
					debug!(target: TW_LOG, "Got new gossip message");
					TSSData::try_from_slice(&notification.message).ok()
				})
				.fuse(),
		);
		loop {
			let receiver = self.sign_data_receiver.clone();
			let mut signature_requests = receiver.lock().await;
			// making sure majority is synchronising or waiting for the sync
			while self.sync_oracle.is_major_syncing() {
				debug!(target: TW_LOG, "Waiting for major sync to complete...");
				futures_timer::Delay::new(Duration::from_secs(1)).await;
			}

			let mut engine = &mut self.gossip_engine;

			futures::select_biased! {
				_ = engine => {
					error!(
						target: TW_LOG,
						"Gossip engine has terminated."
					);
					return;
				},
				storage_notification = self.shard_storage_notifications.next().fuse() => {
					if let Some(notification) = storage_notification {
						self.on_storage_update(notification);
					} else {
						debug!(
							target: TW_LOG,
							"no new storage notifications"
						);
					}
				},
				notification = self.finality_notifications.next().fuse() => {
					if let Some(notification) = notification {
						self.on_finality(notification);
					} else {
						debug!(
							target: TW_LOG,
							"no new finality notifications"
						);
					}
				},
				new_sig = signature_requests.next().fuse() => {
					if let Some((shard_id, data)) = new_sig {
						self.process_sign_message(shard_id, data);
					}
				},
				gossip = gossips.next() => {
					if let Some(gossip) = gossip {
						self.on_gossip(gossip).await;
					} else {
						debug!(
							target: TW_LOG,
							"no new gossips"
						);
					}
				},
			}
		}
	}
}
