#![allow(clippy::type_complexity)]
use crate::{
	communication::validator::{topic, GossipValidator},
	kv::TimeKeyvault,
	tss_event_handler_helper::{aggregator_event_sign, handler_partial_signature_generate_req},
	Client, WorkerParams, TW_LOG,
};
use borsh::{BorshDeserialize, BorshSerialize};
use codec::{Decode, Encode};
use futures::{channel::mpsc::Receiver as FutReceiver, FutureExt, StreamExt};
use log::{debug, error, info, warn};
use sc_client_api::{Backend, FinalityNotification, FinalityNotifications};
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
use time_primitives::{crypto::Public, sharding::Shard, TimeApi, TimeId, KEY_TYPE};
use tokio::sync::Mutex as TokioMutex;
use tss::{
	frost_dalek::{compute_message_hash, SignatureAggregator},
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
	gossip_engine: GossipEngine<B>,
	gossip_validator: Arc<GossipValidator<B>>,
	sign_data_receiver: Arc<TokioMutex<FutReceiver<(u64, [u8; 32])>>>,
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

	// checks if we're present in specific shard
	// at latste finalized block
	fn is_member_of_shard(&self, shard_id: u64) -> bool {
		let at = self.backend.blockchain().last_finalized().unwrap();
		if let Some(id) = self.account_id() {
			let at = BlockId::Hash(at);
			if let Ok(Some(members)) = self.runtime.runtime_api().get_shard_members(&at, shard_id) {
				let am_member = members.iter().any(|s| s == &id);
				debug!(target: TW_LOG, "Am a memmber of shard {}?: {}", shard_id, am_member);
				am_member
			} else {
				error!(
					target: TW_LOG,
					"Failed to fetch members for shard {} from runtime in block {}", shard_id, at
				);
				false
			}
		} else {
			error!(target: TW_LOG, "Failed to construct account");
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
		debug!(target: TW_LOG, "Got new finality notification: {}", notification.header.number());
		let keys = self.kv.public_keys();
		if keys.len() != 1 {
			error!(target: TW_LOG, "Expected exactly one Time key - can not participate.");
			return;
		}
		let our_key = time_primitives::TimeId::decode(&mut keys[0].as_ref()).unwrap();
		debug!(target: TW_LOG, "our key: {}", our_key.to_string());
		let in_block = BlockId::Hash(notification.header.hash());
		// TODO: stabilize unwrap
		let shards = self.runtime.runtime_api().get_shards(&in_block).unwrap();
		debug!(target: TW_LOG, "Read shards from runtime {:?}", shards);
		// check if we're member in new shards
		let new_shards: Vec<_> =
			shards.into_iter().filter(|(id, _)| !self.known_sets.contains(id)).collect();
		for (id, new_shard) in new_shards {
			debug!(
				target: TW_LOG,
				"Checking if a member of shard {} with ID: {}",
				id,
				our_key.to_string()
			);
			if new_shard.members().contains(&our_key) {
				// participate in keygen for new shard
				debug!(target: TW_LOG, "Participating in new keygen for id {}", id);
				// collector starts keygen
				if new_shard.collector() == &our_key {
					self.new_keygen_for_new_id(id, new_shard);
				}
			} else {
				debug!(target: TW_LOG, "Not a member of shard {}", id);
			}
			self.known_sets.insert(id);
		}
	}

	fn process_sign_message(&mut self, shard_id: u64, data: [u8; 32]) {
		// do sig
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			let context = state.context;
			let msg_hash = compute_message_hash(&context, &data);
			//add node in msg_pool
			if state.msg_pool.get(&msg_hash).is_none() {
				state.msg_pool.insert(msg_hash);
				//process msg if req already received
				if let Some(pending_msg_req) = state.msgs_signature_pending.get(&msg_hash) {
					let request = PartialMessageSign {
						msg_hash,
						signers: pending_msg_req.to_vec(),
					};
					let encoded = request.try_to_vec().unwrap();
					if let Some((peer_id, data, message_type)) =
						handler_partial_signature_generate_req(state, shard_id, &encoded)
					{
						if let Ok(encoded_data) = data.try_to_vec() {
							if let Ok(data) =
								make_gossip_tss_data(peer_id, encoded_data, message_type)
							{
								self.gossip_engine.gossip_message(topic::<B>(), data, false);
							} else {
								error!("TSS::error making gossip data for encoded participant");
							}
						} else {
							//log error
							error!("TSS::tss error");
						}
					}
					state.msgs_signature_pending.remove(&msg_hash);
				} else {
					debug!(
						target: TW_LOG,
						"New data for signing received: {:?} with hash {:?}", data, msg_hash
					);
				}
			} else {
				warn!(
					target: TW_LOG,
					"Message with hash {} is already in process of signing",
					hex::encode(msg_hash)
				);
			}
			//creating signature aggregator for msg
			if state.is_node_aggregator {
				if let Some(local_state) = state.local_finished_state.clone() {
					//all nodes should share the same message hash
					//to verify the threshold signature
					let mut aggregator =
						SignatureAggregator::new(state.tss_params, local_state.0, &context, &data);

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
						debug!(target: TW_LOG, "TSS peer collection req sent");
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
		} else {
			warn!(
				target: TW_LOG,
				"Local finished state for given shard ID {} not found!", shard_id
			);
		}
	}

	pub(crate) fn send(&mut self, msg: Vec<u8>) {
		debug!(target: TW_LOG, "Sending new gossip message");
		self.gossip_engine.gossip_message(topic::<B>(), msg, false);
	}

	// Creates opitonal ID from our Time Key
	pub(crate) fn id(&self) -> Option<String> {
		match self.kv.public_keys() {
			keys if !keys.is_empty() => Some(keys[0].to_string()),
			_ => {
				warn!("TSS: No local peer identity ");
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
			let id = &keys[0];
			Some(TimeId::decode(&mut id.as_ref()).unwrap())
		}
	}

	/// On each gossip we process it, verify signature and update our tracker
	async fn on_gossip(&mut self, tss_gossiped_data: TSSData) {
		debug!(target: TW_LOG, "Processing new gossip");
		match tss_gossiped_data.tss_event_type {
			//nodes will be receiving this event to make participant using params
			TSSEventType::ReceiveParams(shard_id) => {
				// check if we're part of given shard
				debug!(target: TW_LOG, "ReceiveParams for shard: {}", shard_id);
				if self.is_member_of_shard(shard_id) {
					debug!(target: TW_LOG, "Am member of {}", shard_id);
					self.handler_receive_params(shard_id, &tss_gossiped_data.tss_data).await;
				}
			},
			// nodes will receive peer id of other nodes and will add it to their list
			TSSEventType::ReceivePeerIDForIndex(shard_id) => {
				debug!(target: TW_LOG, "ReceivePeerIDForIndex for shard: {}", shard_id);
				self.handler_receive_peer_id_for_index(shard_id, &tss_gossiped_data.tss_data)
					.await;
			}, //nodes receives peers with collector participants
			TSSEventType::ReceivePeersWithColParticipant(shard_id) => {
				debug!(target: TW_LOG, "receivePeersWithColParticipant for shard: {}", shard_id);
				self.handler_receiver_peers_with_col_participant(
					shard_id,
					&tss_gossiped_data.tss_data,
				)
				.await;
			},
			//nodes will receive participant and will add will go to round one state
			TSSEventType::ReceiveParticipant(shard_id) => {
				debug!(target: TW_LOG, "ReceiveParticipant for shard: {}", shard_id);
				self.handler_receive_participant(shard_id, &tss_gossiped_data.tss_data).await;
			},
			//nodes will receive their secret share and take state to round two
			TSSEventType::ReceiveSecretShare(shard_id) => {
				debug!(target: TW_LOG, "Received ReceiveSecretShare");
				self.handler_receive_secret_share(shard_id, &tss_gossiped_data.tss_data).await;
			},

			//received commitments of other nodes who are participating in TSS process
			TSSEventType::ReceiveCommitment(shard_id) => {
				debug!(target: TW_LOG, "ReceiveCommitment for shard: {}", shard_id);
				self.handler_receive_commitment(shard_id, &tss_gossiped_data.tss_data).await;
			},

			//event received by collector and partial sign request is received
			TSSEventType::PartialSignatureGenerateReq(shard_id) => {
				debug!(target: TW_LOG, "PartialSignatureGeneratedReq for shard: {}", shard_id);
				if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
					debug!(target: TW_LOG, "Have state for shard: {}", shard_id);
					handler_partial_signature_generate_req(
						state,
						shard_id,
						&tss_gossiped_data.tss_data,
					);
				}
			},

			//received partial signature. threshold signature to be made by aggregator
			TSSEventType::PartialSignatureReceived(shard_id) => {
				debug!(target: TW_LOG, "PartialSignatureReceived for shard: {}", shard_id);
				self.handler_partial_signature_received(shard_id, &tss_gossiped_data.tss_data)
					.await;
			},

			//verify threshold signature generated by aggregator
			TSSEventType::VerifyThresholdSignature(shard_id) => {
				debug!(target: TW_LOG, "VerifyTHresholdSignature for shard: {}", shard_id);
				self.handler_verify_threshold_signature(shard_id, &tss_gossiped_data.tss_data)
					.await;
			},

			//received resetting tss state request
			TSSEventType::ResetTSSState(shard_id) => {
				debug!(target: TW_LOG, "ResetTSSSTate for shard: {}", shard_id);
				self.handler_reset_tss_state(shard_id, &tss_gossiped_data.tss_data).await;
			},
		}
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
