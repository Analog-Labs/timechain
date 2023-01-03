use crate::{
	local_state_struct::TSSLocalStateData,
	submit_to_timechain::{
		timechain::runtime_types::pallet_tesseract_sig_storage::types::TesseractRole,
		TimechainSubmitter,
	},
	tss_event_model::{PartialMessageSign, TSSData, TSSEventType, TSSLocalStateType},
	utils::{get_receive_params_msg, get_reset_tss_msg, make_gossip_tss_data},
	DEFUALT_TSS_TOTAL_NODES,
};
use accounts::Account;
use borsh::BorshSerialize;
use frost_dalek::signature::Signer;

use frost_dalek::{compute_message_hash, Parameters, SignatureAggregator};
use keystore::commands::KeyTypeId;
use sp_keystore::SyncCryptoStore;
use std::sync::Arc;
use tokio::{
	sync::mpsc::{Receiver, Sender},
	time,
};
pub const MIN_THRESHOLD_PERCENTAGE: u32 = 75;
pub const COLLECTOR_ADDR: &str = "";

pub struct TssService {
	pub gossip_to_tss_receiver: Receiver<TSSData>,
	pub tss_to_gossip_sender: Sender<Vec<u8>>,
	pub tss_local_state: TSSLocalStateData,
	pub event_receiver: Receiver<String>,
	pub account: Account,
	pub config: Arc<TimechainSubmitter>,
}

impl TssService {
	pub async fn new(
		gossip_to_tss_receiver: Receiver<TSSData>,
		tss_to_gossip_sender: Sender<Vec<u8>>,
		event_receiver: Receiver<String>,
		account: Account,
		//collector patch
		is_default_node: bool,
		peer_id: String,
		tss_nodes_and_threshold_nodes: (u32, u32),
		key_type: Option<KeyTypeId>,
		keystore_option: Option<Arc<dyn SyncCryptoStore>>,
		config: Arc<TimechainSubmitter>,
	) -> Self {
		let mut unlocked_state = TSSLocalStateData::new();
		if !COLLECTOR_ADDR.is_empty() {
			if COLLECTOR_ADDR.eq(&peer_id) {
				unlocked_state.is_node_collector = true;
			} else {
				unlocked_state.is_node_collector = false;
			}
		} else {
			unlocked_state.is_node_collector = is_default_node;
		}

		unlocked_state.local_peer_id = Some(peer_id);
		unlocked_state.key_type = key_type;
		unlocked_state.keystore = keystore_option;

		if tss_nodes_and_threshold_nodes.0 >= DEFUALT_TSS_TOTAL_NODES {
			//stop if total nodes for tss provided and threshold number if invalid
			assert!(tss_nodes_and_threshold_nodes.1 >= 1_u32, "Invalid threshold nodes provided");

			unlocked_state.tss_params = Parameters {
				n: tss_nodes_and_threshold_nodes.0,
				t: tss_nodes_and_threshold_nodes.1,
			};
		}

		Self {
			gossip_to_tss_receiver,
			tss_to_gossip_sender,
			tss_local_state: unlocked_state,
			event_receiver,
			account,
			config,
		}
	}
	pub async fn run(&mut self) {
		let mut timer = time::interval(time::Duration::from_secs(5));

		//add aggregator and collector
		if self.tss_local_state.is_node_collector {
			match self.config.add_member(TesseractRole::Collector).await {
				Ok(_) => {
					log::info!("Collector added successfully to timechain");
				},
				Err(e) => {
					log::error!("Error adding Collector to timechain: {:?}", e)
				},
			};
		} else {
			match self.config.add_member(TesseractRole::Aggregator).await {
				Ok(_) => {
					log::info!("Aggregator added successfully to timechain");
				},
				Err(e) => {
					log::error!("Error adding Aggregator to timechain: {:?}", e)
				},
			};
		}

		loop {
			tokio::select! {
				//received event from network
				r = self.gossip_to_tss_receiver.recv() => {
					if let Some(data) = r{
						log::info!("=====================");
						log::info!("received tss gossip from peer: {:?} for {:?}", data.peer_id, data.tss_event_type);
						log::info!("=====================");
						self.handle_tss_events(data).await;

					}
				}

				//if event is receiver from connector side then publish for signing
				event_receiver = self.event_receiver.recv() => {
					// let tss_local_state = self.tss_local_state;
					let local_peer_id = self.tss_local_state.local_peer_id.clone().unwrap();

					if let Some(data) = event_receiver{
						log::info!("got event to tss {:?}", data);
						let context = self.tss_local_state.context;
						let msg_hash = compute_message_hash(&context, data.as_bytes());

						//add node in msg_pool
						if !self.tss_local_state.msg_pool.contains_key(&msg_hash){
							self.tss_local_state.msg_pool.insert(msg_hash, data.clone().into());

							//process msg if req already received
							if let Some(pending_msg_req) = self.tss_local_state.msgs_signature_pending.get(&msg_hash){
								self.process_pending_msg_req(msg_hash, pending_msg_req.to_vec()).await;
								self.tss_local_state.msgs_signature_pending.remove(&msg_hash);
							}else{
								log::info!("msg not pending request for data {:?} and hash {:?}", data, msg_hash);
							}

						}else{
							log::warn!("Msg already in pool");
						}

						//creating signature aggregator for msg
						if self.tss_local_state.is_node_aggregator{

							//all nodes should share the same message hash
							//to verify the threshold signature
							let mut aggregator = SignatureAggregator::new(
								self.tss_local_state.tss_params,
								self.tss_local_state.local_finished_state.clone().unwrap().0,
								&context,
								data.as_bytes(),
							);

							for com in self.tss_local_state.others_commitment_share.clone(){
								aggregator.include_signer(
									com.public_commitment_share_list.participant_index,
									com.public_commitment_share_list.commitments[0],
									com.public_key,
								);
							}

							//including aggregator as a signer
							aggregator.include_signer(
								self.tss_local_state.local_index.unwrap(),
								self.tss_local_state.local_commitment_share.clone().unwrap().0.commitments[0],
								self.tss_local_state.local_public_key.clone().unwrap(),
							);

							//this signers list will be used by other nodes to verify themselves.
							let signers = aggregator.get_signers();
							self.tss_local_state.current_signers = signers.clone();

							// //sign msg from aggregator side
							self.aggregator_event_sign(msg_hash).await;

							let sign_msg_req = PartialMessageSign{
								msg_hash,
								signers: signers.clone(),
							};

							self.publish_to_network(local_peer_id.clone(),
								sign_msg_req,
								TSSEventType::PartialSignatureGenerateReq,
						).await;

						}
					}else{
						log::error!("No data received from event receiver");
					}

				}

				//time loop to start tss process
				_ = timer.tick() => {
					//collector node starting TSS process
					if let Some(local_peer_id) = self.tss_local_state.local_peer_id.clone(){
						if self.tss_local_state.is_node_collector && self.tss_local_state.tss_process_state <= TSSLocalStateType::ReceivedPeers{

							//sending reset state request to all nodes since didn't received good amount of nodes.
							self.tss_local_state.reset();
							self.tss_local_state.is_node_collector = true;
							if let Ok(reset_call) = get_reset_tss_msg("Reinit state".into()){
								if let Ok(reset_data) = make_gossip_tss_data(local_peer_id.clone(), reset_call, TSSEventType::ResetTSSState){
									if let Err(e) = self.tss_to_gossip_sender.send(reset_data).await{
										log::error!("error sending TSS reset request to gossip: {:?}", e);
									}
								}
							}

							//sending gossip to start tss initialization process
							if let Ok(peer_id_data) = get_receive_params_msg(local_peer_id.clone(), self.tss_local_state.tss_params){
								if let Ok(data) = make_gossip_tss_data(local_peer_id.clone(), peer_id_data, TSSEventType::ReceiveParams){
									let _ = self.tss_to_gossip_sender.send(data).await;
									log::info!("TSS peer collection req sent");
								}
							}else{
								log::error!("Unable to make publish peer id msg");
							}
							log::info!("current node state {:#?}", self.tss_local_state);
						}
					}
				}
				//can pool other futures here
			}
		}
	}

	pub async fn process_pending_msg_req(
		&mut self,
		msg_hash: [u8; 64],
		signer_for_msg: Vec<Signer>,
	) {
		//create req object
		let req = PartialMessageSign {
			msg_hash,
			signers: signer_for_msg.clone(),
		};

		if let Ok(encrypted_data) = req.try_to_vec() {
			self.handler_partial_signature_generate_req(&encrypted_data).await;
		} else {
			log::error!("Unable to send pending msg request: ecryption failed");
		}
	}
}
