use crate::{
	communication::validator::{topic, GossipValidator},
	inherents::update_shared_group_key,
	kv::TimeKeyvault,
	Client, WorkerParams, TW_LOG,
};
use borsh::{BorshDeserialize, BorshSerialize};
use futures::{future, FutureExt, StreamExt};
use log::{debug, error, info, warn};
use parking_lot::Mutex;
use sc_client_api::{Backend, FinalityNotification, FinalityNotifications};
use sc_network_gossip::GossipEngine;
use sp_api::ProvideRuntimeApi;
use sp_consensus::SyncOracle;
use sp_runtime::traits::{Block, Header};
use std::{sync::Arc, time::Duration};
use time_primitives::{TimeApi, KEY_TYPE};
use tss::{
	local_state_struct::TSSLocalStateData,
	tss_event_model::{TSSData, TSSEventType},
	utils::get_reset_tss_msg,
};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct TimeWorker<B: Block, C, R, BE, SO> {
	client: Arc<C>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	finality_notifications: FinalityNotifications<B>,
	gossip_engine: Arc<Mutex<GossipEngine<B>>>,
	gossip_validator: Arc<GossipValidator<B>>,
	pub(crate) kv: TimeKeyvault,
	sync_oracle: SO,
	pub(crate) tss_local_state: TSSLocalStateData,
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
		} = worker_params;

		let mut tss_local_state = TSSLocalStateData::new();
		tss_local_state.key_type = Some(KEY_TYPE);
		tss_local_state.keystore = kv.get_store();
		// TODO: threshold calc if required

		TimeWorker {
			finality_notifications: client.finality_notification_stream(),
			client,
			backend,
			runtime,
			gossip_engine: Arc::new(Mutex::new(gossip_engine)),
			gossip_validator,
			sync_oracle,
			kv,
			tss_local_state,
		}
	}

	/// On each grandpa finality we're initiating gossip to all other authorities to acknowledge
	fn on_finality(&mut self, notification: FinalityNotification<B>) {
		info!(target: TW_LOG, "Got new finality notification: {}", notification.header.number());
		let _number = notification.header.number();
		let keys = self.kv.public_keys();
		if keys.is_empty() {
			warn!(target: TW_LOG, "No time key found, please inject one.");
			return;
		}
		let id = keys[0].clone();
		// starting TSS initialization if not yet done
		if self.tss_local_state.local_peer_id.is_none() {
			self.tss_local_state.local_peer_id = Some(id.to_string());
			let msg = TSSData {
				peer_id: id.to_string(),
				tss_event_type: TSSEventType::ReceiveParams,
				tss_data: get_reset_tss_msg("Reinit state".to_string()).unwrap_or_default(),
			};
			let mut enc_message = vec![];
			msg.serialize(&mut enc_message).unwrap_or(());
			self.gossip_engine.lock().gossip_message(topic::<B>(), enc_message, false);
			info!(target: TW_LOG, "Gossip is sent.");
		}
	}

	pub(crate) fn send(&self, msg: Vec<u8>) {
		self.gossip_engine.lock().gossip_message(topic::<B>(), msg, false);
	}

	#[allow(dead_code)]
	// Using this method each validator can, and should, submit shared `GroupKey` key to runtime
	fn submit_key_as_inherent(&self, key: [u8; 32], set_id: u64) {
		update_shared_group_key(set_id, key);
	}

	/// On each gossip we process it, verify signature and update our tracker
	async fn on_gossip(&mut self, tss_gossiped_data: TSSData) {
		match tss_gossiped_data.tss_event_type {
			//nodes will be receiving this event to make participant using params
			TSSEventType::ReceiveParams => {
				self.handler_receive_params(&tss_gossiped_data.tss_data).await;
			},
			// nodes will receive peer id of other nodes and will add it to their list
			TSSEventType::ReceivePeerIDForIndex => {
				self.handler_receive_peer_id_for_index(&tss_gossiped_data.tss_data).await;
			}, //nodes receives peers with collector participants
			TSSEventType::ReceivePeersWithColParticipant => {
				self.handler_receiver_peers_with_col_participant(&tss_gossiped_data.tss_data)
					.await;
			},
			//nodes will receive participant and will add will go to round one state
			TSSEventType::ReceiveParticipant => {
				self.handler_receive_participant(&tss_gossiped_data.tss_data).await;
			},
			//nodes will receive their secret share and take state to round two
			TSSEventType::ReceiveSecretShare => {
				info!(target: TW_LOG, "Received ReceiveSecretShare");
				self.handler_receive_secret_share(&tss_gossiped_data.tss_data).await;
			},

			//received commitments of other nodes who are participating in TSS process
			TSSEventType::ReceiveCommitment => {
				self.handler_receive_commitment(&tss_gossiped_data.tss_data).await;
			},

			//event received by collector and partial sign request is received
			TSSEventType::PartialSignatureGenerateReq => {
				self.handler_partial_signature_generate_req(&tss_gossiped_data.tss_data).await;
			},

			//received partial signature. threshold signature to be made by aggregator
			TSSEventType::PartialSignatureReceived => {
				self.handler_partial_signature_received(&tss_gossiped_data.tss_data).await;
			},

			//verify threshold signature generated by aggregator
			TSSEventType::VerifyThresholdSignature => {
				self.handler_verify_threshold_signature(&tss_gossiped_data.tss_data).await;
			},

			//received resetting tss state request
			TSSEventType::ResetTSSState => {
				self.handler_reset_tss_state(&tss_gossiped_data.tss_data).await;
			},
		}
	}

	/// Our main worker main process - we act on grandpa finality and gossip messages for interested
	/// topics
	pub(crate) async fn run(&mut self) {
		let mut gossips =
			Box::pin(self.gossip_engine.lock().messages_for(topic::<B>()).filter_map(
				|notification| async move {
					debug!(target: TW_LOG, "Got new gossip message");
					TSSData::deserialize(&mut &notification.message[..]).ok()
				},
			));
		loop {
			// making sure majority is synchronising or waiting for the sync
			while self.sync_oracle.is_major_syncing() {
				debug!(target: TW_LOG, "Waiting for major sync to complete...");
				futures_timer::Delay::new(Duration::from_secs(1)).await;
			}

			let engine = self.gossip_engine.clone();
			let gossip_engine = future::poll_fn(|cx| engine.lock().poll_unpin(cx));

			futures::select! {
				gossip = gossips.next().fuse() => {
					if let Some(gossip) = gossip {
						self.on_gossip(gossip).await;
					} else {
						debug!(
							target: TW_LOG,
							"no new gossips"
						);
						continue;
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
						continue;
					}
				},
				_ = gossip_engine.fuse() => {
					error!(
						target: TW_LOG,
						"Gossip engine has terminated."
					);
					return;
				}
			}
		}
	}
}
