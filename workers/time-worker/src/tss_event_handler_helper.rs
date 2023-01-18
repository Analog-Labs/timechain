use crate::{traits::Client, worker::TimeWorker};
use accounts::Account;
use borsh::{BorshDeserialize, BorshSerialize};
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_consensus::SyncOracle;
use sp_runtime::traits::Block;
use std::collections::HashMap;
use time_primitives::TimeApi;
use tss::rand::rngs::OsRng;

use tss::{
	frost_dalek::{
		generate_commitment_share_lists, keygen::SecretShare, Participant, SignatureAggregator,
	},
	signverify::sign_data,
	tss_event_model::{
		FilterAndPublishParticipant, OthersCommitmentShares, PartialMessageSign, PublishPeerIDCall,
		ReceiveParamsWithPeerCall, ReceivePartialSignatureReq, ResetTSSCall, TSSEventType,
		TSSLocalStateType, VerifyThresholdSignatureReq,
	},
	utils::{
		get_participant_index, get_publish_peer_id_msg, make_gossip_tss_data,
		make_hashmap_for_secret_share, make_participant, round_one_state,
	},
};

impl<B, C, R, BE, SO> TimeWorker<B, C, R, BE, SO>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B>,
	SO: SyncOracle + Send + Sync + Clone + 'static,
{
	//will be run by non collector nodes
	pub async fn handler_receive_params(self: &mut Self, data: &Vec<u8>) {
		let local_peer_id = match self.tss_local_state.local_peer_id.clone() {
			Some(id) => id,
			None => match self.kv.public_keys() {
				keys if keys.len() > 0 => keys[0].to_string(),
				_ => {
					log::warn!(
						"TSS: No local peer identity present for received params processing"
					);
					return;
				},
			},
		};

		if self.tss_local_state.tss_process_state == TSSLocalStateType::Empty {
			if let Ok(peer_id_call) = ReceiveParamsWithPeerCall::try_from_slice(&data) {
				self.tss_local_state.tss_params = peer_id_call.params;
				self.tss_local_state.tss_process_state = TSSLocalStateType::ReceivedParams;

				let peer_id = peer_id_call.peer_id;
				if !self.tss_local_state.others_peer_id.contains(&peer_id) {
					self.tss_local_state.others_peer_id.push(peer_id);
				}

				if let Ok(peer_id_data) = get_publish_peer_id_msg(local_peer_id.clone()) {
					//nodes replies to this event with their peer id
					if let Ok(data) = make_gossip_tss_data(
						local_peer_id,
						peer_id_data,
						TSSEventType::ReceivePeerIDForIndex,
					) {
						log::debug!("TSS: Sending ReceivePeerIDForIndex");
						self.send(data.into());
					} else {
						log::error!("TSS::Unable to encode gossip data for participant creation");
					}
				} else {
					log::error!("TSS::Unable to get publish peer id msg");
				}
			} else {
				log::error!("TSS::Could not deserialize params");
			}
		} else {
			log::error!(
				"TSS::Received params but node is not in empty state {:?}",
				self.tss_local_state.tss_process_state
			);
		}
	}

	//used by node collector to set peers for tss process
	pub async fn handler_receive_peer_id_for_index(self: &mut Self, data: &Vec<u8>) {
		let local_peer_id = self.tss_local_state.local_peer_id.clone().unwrap();

		//receive index and update state of node
		if self.tss_local_state.is_node_collector {
			if self.tss_local_state.tss_process_state == TSSLocalStateType::Empty {
				if let Ok(peer_id_call) = PublishPeerIDCall::try_from_slice(data) {
					let peer_id = peer_id_call.peer_id;

					if !self.tss_local_state.others_peer_id.contains(&peer_id) {
						self.tss_local_state.others_peer_id.push(peer_id);

						let params = self.tss_local_state.tss_params;

						//check if we have min number of nodes
						if self.tss_local_state.others_peer_id.len() >= (params.n as usize - 1) {
							//change connector node state to received peers
							self.tss_local_state.tss_process_state =
								TSSLocalStateType::ReceivedPeers;

							let mut other_peer_list = self.tss_local_state.others_peer_id.clone();
							let index =
								get_participant_index(local_peer_id.clone(), &other_peer_list);
							self.tss_local_state.local_index = Some(index);

							//collector node making participant and publishing
							let participant = make_participant(params, index);
							self.tss_local_state.local_participant = Some(participant.clone());

							log::info!("TSS::this nodes participant index {}", index);

							//preparing publish data
							other_peer_list
								.push(self.tss_local_state.local_peer_id.clone().unwrap());
							let data = FilterAndPublishParticipant {
								total_peer_list: other_peer_list,
								col_participant: participant.0,
							};

							//publish to network
							self.publish_to_network::<FilterAndPublishParticipant>(
								local_peer_id,
								data,
								TSSEventType::ReceivePeersWithColParticipant,
							)
							.await;
						}
					}
				} else {
					log::error!("TSS::PeerID already exists in local state list");
				}
			} else {
				log::error!("TSS::Received peer id for index but node is not in empty state");
			}
		}
	}

	//filter participants and publish participants to network
	pub async fn handler_receiver_peers_with_col_participant(self: &mut Self, data: &Vec<u8>) {
		let local_peer_id = self.tss_local_state.local_peer_id.clone().unwrap();
		if self.tss_local_state.tss_process_state == TSSLocalStateType::ReceivedParams {
			if let Ok(data) = FilterAndPublishParticipant::try_from_slice(data) {
				let mut other_peer_list = data.total_peer_list;

				if let Some(index) = other_peer_list.iter().position(|x| x.eq(&local_peer_id)) {
					other_peer_list.remove(index);
					self.tss_local_state.tss_process_state = TSSLocalStateType::ReceivedPeers;
				} else {
					self.tss_local_state.tss_process_state = TSSLocalStateType::NotParticipating;
					return;
				}

				if !self.tss_local_state.others_participants.contains(&data.col_participant) {
					self.tss_local_state.others_participants.push(data.col_participant);
				}

				self.tss_local_state.others_peer_id = other_peer_list.clone();
				let index = get_participant_index(local_peer_id.clone(), &other_peer_list);
				self.tss_local_state.local_index = Some(index);

				//make participant and publish
				let participant = make_participant(self.tss_local_state.tss_params, index);
				self.tss_local_state.local_participant = Some(participant.clone());

				self.publish_to_network(
					local_peer_id,
					participant.0,
					TSSEventType::ReceiveParticipant,
				)
				.await;
			}
		} else {
			log::error!("TSS::Received peers with col participant but node is not in empty state");
		}
	}

	//receive participant and publish to network secret share to network when all participants are
	// received
	pub async fn handler_receive_participant(self: &mut Self, data: &Vec<u8>) {
		let local_peer_id = self.tss_local_state.local_peer_id.clone().unwrap();
		//receive participants and update state of node
		if self.tss_local_state.tss_process_state == TSSLocalStateType::ReceivedPeers {
			if let Ok(participant) = Participant::try_from_slice(data) {
				if !self.tss_local_state.others_participants.contains(&participant) {
					self.tss_local_state.others_participants.push(participant);
				}

				let params = self.tss_local_state.tss_params;
				let total_nodes = params.n;
				let other_nodes = (self.tss_local_state.others_participants.len() + 1) as u32;
				if total_nodes == other_nodes {
					// received total participants proceed to next process
					// creating secret share etc
					let local_index = match self.tss_local_state.local_index {
						Some(index) => index,
						None => {
							log::error!("TSS::local index not found");
							return;
						},
					};
					let participant = match &self.tss_local_state.local_participant.clone() {
						Some(participant) => participant.clone(),
						None => {
							log::error!("TSS::local participant not found");
							return;
						},
					};
					if let Ok(round_one_state) = round_one_state(
						&params,
						&local_index,
						&participant.1,
						&mut self.tss_local_state.others_participants,
					) {
						//making hashmap of our their_secret_share
						let secret_shares = match round_one_state.their_secret_shares() {
							Ok(secret_shares) => secret_shares,
							Err(e) => {
								log::error!("TSS::error getting secret shares: {:#?}", e);
								return;
							},
						};

						//making hash table for secret share with index
						let distributed_hashmap =
							make_hashmap_for_secret_share(secret_shares).await;

						self.tss_local_state.local_dkg_r1_state = Some(round_one_state.clone());

						self.tss_local_state.tss_process_state = TSSLocalStateType::DkgGeneratedR1;
						log::info!("TSS::Keygen phase 1 done");

						//publish everyone's secret share to network
						self.publish_to_network::<HashMap<u32, SecretShare>>(
							local_peer_id,
							distributed_hashmap.clone(),
							TSSEventType::ReceiveSecretShare,
						)
						.await;
					} else {
						log::error!("TSS::error in generating round one state");
					}
				}
			} else {
				//log error
				log::error!("TSS::Error deserializing participant");
			}
		} else {
			log::error!(
				"TSS::Received participant but node is not in correct state: {:?}",
				self.tss_local_state.tss_process_state
			);
		}
	}

	//receives secret share form all other nodes which are participating in the tss
	pub async fn handler_receive_secret_share(self: &mut Self, data: &Vec<u8>) {
		let local_peer_id = self.tss_local_state.local_peer_id.clone().unwrap();
		//receive secret shares and update state of node
		if self.tss_local_state.tss_process_state == TSSLocalStateType::DkgGeneratedR1 {
			if let Ok(distributed_hashmap) = HashMap::<u32, SecretShare>::try_from_slice(data) {
				let local_index = match self.tss_local_state.local_index {
					Some(index) => index,
					None => {
						log::error!("TSS::unable to get local index");
						return;
					},
				};
				if let Some(secret_share) = distributed_hashmap.get(&local_index) {
					if !self.tss_local_state.others_my_secret_share.contains(secret_share) {
						self.tss_local_state.others_my_secret_share.push(secret_share.clone());
					}

					let others_my_secret_shares =
						self.tss_local_state.others_my_secret_share.clone();
					let params = self.tss_local_state.tss_params;
					let total_nodes = params.n;
					let other_nodes = self.tss_local_state.others_my_secret_share.len() as u32;
					if total_nodes == other_nodes + 1 {
						let round_one = match self.tss_local_state.local_dkg_r1_state.clone() {
							Some(round_one) => round_one,
							None => {
								log::error!("TSS::Could not get round one state from local state");
								return;
							},
						};
						match round_one.to_round_two(others_my_secret_shares) {
							Ok(round_two_state) => {
								self.tss_local_state.local_dkg_r2_state =
									Some(round_two_state.clone());

								self.tss_local_state.tss_process_state =
									TSSLocalStateType::DkgGeneratedR2;
								log::info!("TSS::Keygen phase 2 done");

								//finish local state progress
								let participant =
									match self.tss_local_state.local_participant.clone() {
										Some(participant) => participant,
										None => {
											log::error!(
                                        "TSS::Unable to get local participant from local state"
                                    );
											return;
										},
									};
								let my_commitment =
									match participant.0.public_key() {
										Some(commitment) => commitment,
										None => {
											log::error!("TSS::Unable to get commitment from local participant");
											return;
										},
									};
								if let Ok((local_group_key, local_secret_key)) =
									round_two_state.finish(my_commitment)
								{
									//update local state
									self.tss_local_state.local_finished_state =
										Some((local_group_key, local_secret_key.clone()));
									self.tss_local_state.local_public_key =
										Some(local_secret_key.to_public());
									self.tss_local_state.tss_process_state =
										TSSLocalStateType::StateFinished;

									log::info!("TSS::==========================");
									let bytes = local_group_key.to_bytes();
									log::info!("TSS::local group key is: {:?}", bytes);
									// TODO: provide set ID from pre-set
									self.submit_key_as_inherent(bytes, 1);
									log::info!("TSS::==========================");
								} else {
									log::error!("TSS::error occured while finishing state");
								}

								//generating and publishing commitment to include node in tss
								// process
								let index = match self.tss_local_state.local_index {
									Some(index) => index,
									None => {
										log::error!("TSS::unable to get local index");
										return;
									},
								};
								if self.tss_local_state.tss_process_state
									== TSSLocalStateType::StateFinished
								{
									//aggregator patch
									self.tss_local_state.is_node_aggregator =
										self.tss_local_state.is_node_collector;

									let local_commitment =
										generate_commitment_share_lists(&mut OsRng, index, 1);

									self.tss_local_state.local_commitment_share =
										Some(local_commitment.clone());

									let pubkey = match self.tss_local_state.local_public_key.clone()
									{
										Some(pubkey) => pubkey,
										None => {
											log::error!(
                                            "TSS::Unable to get local public key from local state"
                                        );
											return;
										},
									};

									let share_commitment = OthersCommitmentShares {
										public_key: pubkey,
										public_commitment_share_list: local_commitment.0.clone(),
									};

									//publish publicCommitmentSharelist to network
									self.publish_to_network(
										local_peer_id,
										share_commitment,
										TSSEventType::ReceiveCommitment,
									)
									.await;
								}
							},
							Err(e) => {
								log::error!("TSS::Error in round two state: {:#?}", e);
								return;
							},
						}
					} else {
						log::info!("TSS::Waiting for other nodes secret share");
					}
				} else {
					log::error!("TSS::Could not get secret share hash table");
				}
			} else {
				log::error!("TSS::Unable to deserialize secret share TSS Params");
			}
		} else {
			log::error!(
				"TSS::Received secret share but node not in correct state {:?}",
				self.tss_local_state.tss_process_state
			);
		}
	}

	//This call is received by aggregators to make list of commitment of participants
	pub async fn handler_receive_commitment(self: &mut Self, data: &Vec<u8>) {
		//receive commitments and update state of node
		if self.tss_local_state.is_node_aggregator {
			if self.tss_local_state.tss_process_state >= TSSLocalStateType::DkgGeneratedR1
				&& self.tss_local_state.tss_process_state <= TSSLocalStateType::StateFinished
			{
				if let Ok(commitment) = OthersCommitmentShares::try_from_slice(data) {
					if !self.tss_local_state.others_commitment_share.contains(&commitment) {
						self.tss_local_state.others_commitment_share.push(commitment);
					}

					let params = self.tss_local_state.tss_params;
					if self.tss_local_state.others_commitment_share.len() == (params.n - 1) as usize
					{
						log::info!("TSS::Received all commitments");
						self.tss_local_state.tss_process_state =
							TSSLocalStateType::CommitmentsReceived;
					} else {
						log::info!(
							"TSS::Not enough commitments, Got {}, Needed {}",
							self.tss_local_state.others_commitment_share.len(),
							params.n - 1
						);
					}
				} else {
					log::error!("TSS::Unable to deserialize commitment");
				}
			} else {
				log::error!("TSS::Received commitment but node not in correct state");
			}
		} else {
			log::info!(
				"TSS::current tss state for receiving commitment is: {:?} with peer id: {:?}",
				self.tss_local_state.tss_process_state,
				self.tss_local_state.local_peer_id
			);
		}
	}

	//This call is received by participant to generate its partial signature
	pub async fn handler_partial_signature_generate_req(self: &mut Self, data: &Vec<u8>) {
		let local_peer_id = self.tss_local_state.local_peer_id.clone().unwrap();

		if self.tss_local_state.tss_process_state >= TSSLocalStateType::StateFinished {
			if let Ok(msg_req) = PartialMessageSign::try_from_slice(data) {
				let final_state = match self.tss_local_state.local_finished_state.clone() {
					Some(final_state) => final_state,
					None => {
						log::error!("TSS::Unable to get local finished state from local state");
						return;
					},
				};

				let mut my_commitment = match self.tss_local_state.local_commitment_share.clone() {
					Some(commitment) => commitment,
					None => {
						log::error!("TSS::Unable to get local commitment share from local state");
						return;
					},
				};

				if let Some(_) = self.tss_local_state.msg_pool.get(&msg_req.msg_hash) {
					//making partial signature here
					let partial_signature = match final_state.1.sign(
						&msg_req.msg_hash,
						&final_state.0,
						&mut my_commitment.1,
						0,
						&msg_req.signers,
					) {
						Ok(partial_signature) => partial_signature,
						Err(e) => {
							log::error!("TSS::error occured while signing: {:?}", e);
							return;
						},
					};

					let gossip_data = ReceivePartialSignatureReq {
						msg_hash: msg_req.msg_hash.clone(),
						partial_sign: partial_signature,
					};

					//publish partial signature to network
					self.publish_to_network(
						local_peer_id,
						gossip_data,
						TSSEventType::PartialSignatureReceived,
					)
					.await;
				} else {
					log::warn!(
						"TSS::data received for signing but not in local pool: {:?}",
						msg_req.msg_hash
					);
					self.tss_local_state
						.msgs_signature_pending
						.insert(msg_req.msg_hash, msg_req.signers);
				}
			} else {
				log::error!("TSS::Unable to deserialize PartialMessageSign");
			}
		} else {
			log::error!("TSS::Node not in correct state to generate partial signature");
		}
	}

	//this call is received by aggregator to make the threshold signature
	pub async fn handler_partial_signature_received(self: &mut Self, data: &Vec<u8>) {
		let local_peer_id = self.tss_local_state.local_peer_id.clone().unwrap();

		//check if aggregator
		if self.tss_local_state.is_node_aggregator {
			if self.tss_local_state.tss_process_state == TSSLocalStateType::CommitmentsReceived {
				if let Ok(msg_req) = ReceivePartialSignatureReq::try_from_slice(data) {
					if let Some(msg) = self.tss_local_state.msg_pool.get(&msg_req.msg_hash) {
						//add in list
						if let Some(hashmap) =
							self.tss_local_state.others_partial_signature.get_mut(&msg_req.msg_hash)
						{
							hashmap.push(msg_req.partial_sign);
						} else {
							let mut participant_list = Vec::new();
							participant_list.push(msg_req.partial_sign);
							self.tss_local_state
								.others_partial_signature
								.insert(msg_req.msg_hash, participant_list);
						}

						let params = self.tss_local_state.tss_params;
						//the unwrap wont fail since we are already adding item above
						if self
							.tss_local_state
							.others_partial_signature
							.get(&msg_req.msg_hash)
							.unwrap()
							.len() == self.tss_local_state.current_signers.len()
						{
							let context = self.tss_local_state.context.clone();
							let finished_state =
								match self.tss_local_state.local_finished_state.clone() {
									Some(finished_state) => finished_state,
									None => {
										log::error!(
                                    "TSS::Unable to get local finished state from local state"
                                );
										return;
									},
								};
							let mut aggregator = SignatureAggregator::new(
								params,
								finished_state.0,
								&context,
								&msg[..],
							);

							for com in self.tss_local_state.others_commitment_share.clone() {
								aggregator.include_signer(
									com.public_commitment_share_list.participant_index,
									com.public_commitment_share_list.commitments[0],
									com.public_key,
								);
							}

							aggregator.include_signer(
								self.tss_local_state.local_index.clone().unwrap(),
								self.tss_local_state
									.local_commitment_share
									.clone()
									.unwrap()
									.0
									.commitments[0],
								self.tss_local_state.local_public_key.clone().unwrap(),
							);

							//include partial signature
							for item in self
								.tss_local_state
								.others_partial_signature
								.get(&msg_req.msg_hash)
								.unwrap()
								.clone()
							{
								aggregator.include_partial_signature(item.clone());
							}

							//finalize aggregator
							let aggregator_finalized = match aggregator.finalize() {
								Ok(aggregator_finalized) => aggregator_finalized,
								Err(e) => {
									for (key, value) in e.into_iter() {
										//These issues are from the aggregator side and not the
										// signer side
										log::error!("TSS::error occured while finalizing aggregator from index {:?} because of {:?}", key, value);
									}
									return;
								},
							};

							//aggregate aggregator
							let threshold_signature = match aggregator_finalized.aggregate() {
								Ok(threshold_signature) => threshold_signature,
								Err(e) => {
									for (key, value) in e.into_iter() {
										//can also send the indices of participants to timechain to
										// keep track of that
										log::error!(
											"TSS::Participant {} misbehaved because {}",
											key,
											value
										);
									}
									return;
								},
							};

							//check for validity of event
							match threshold_signature
								.verify(&finished_state.0, &msg_req.msg_hash.into())
							{
								Ok(_) => {
									log::info!("TSS::Signature is valid sending to network");

									let gossip_data = VerifyThresholdSignatureReq {
										// msg: msg_req.msg,
										msg_hash: msg_req.msg_hash.into(),
										threshold_sign: threshold_signature,
									};

									self.publish_to_network(
										local_peer_id,
										gossip_data,
										TSSEventType::VerifyThresholdSignature,
									)
									.await;
								},
								Err(_) => {
									log::error!("TSS::Signature computed is invalid");
								},
							}

							//reset partial signature
							self.tss_local_state.others_partial_signature.remove(&msg_req.msg_hash);

							//remove event from msg_pool
							self.tss_local_state.msg_pool.remove(&msg_req.msg_hash);
						}
					} else {
						log::error!(
							"TSS::data received for signature but msg not in local pool {:?}",
							self.tss_local_state.msg_pool.len()
						);
					}
				}
			} else {
				log::error!("TSS::Node not in correct state to receive partial signature");
			}
		}
	}

	//This call is received by participant to verify the threshold signature
	pub async fn handler_verify_threshold_signature(self: &mut Self, data: &Vec<u8>) {
		if self.tss_local_state.tss_process_state >= TSSLocalStateType::StateFinished {
			if let Ok(threshold_signature) = VerifyThresholdSignatureReq::try_from_slice(data) {
				if let Some(_) = self.tss_local_state.msg_pool.get(&threshold_signature.msg_hash) {
					let finished_state = match self.tss_local_state.local_finished_state.clone() {
						Some(finished_state) => finished_state,
						None => {
							log::error!("TSS::Unable to get local finished state from local state");
							return;
						},
					};
					match threshold_signature
						.threshold_sign
						.verify(&finished_state.0, &threshold_signature.msg_hash.into())
					{
						Ok(_) => {
							//remove event from msg_pool
							self.tss_local_state.msg_pool.remove(&threshold_signature.msg_hash);
							log::info!(
								"length of msg_pool {:?}",
								self.tss_local_state.msg_pool.len()
							);
						},
						Err(e) => {
							log::error!("TSS::Could not verify signature: {:?}", e);
						},
					}
				} else {
					log::error!("TSS::Could not find message in local pool for verification");
				}
			} else {
				log::error!("TSS::Could not deserialize VerifiyThresholdSignatureReq");
			}
		} else {
			log::error!(
				"TSS::Node not in correct state to verify threshold signature, {:?}",
				self.tss_local_state.tss_process_state
			);
		}
	}

	//This call resets the tss state data to empty/initial state
	pub async fn handler_reset_tss_state(self: &mut Self, data: &Vec<u8>) {
		//reset TSS State
		if let Ok(data) = ResetTSSCall::try_from_slice(data) {
			log::error!("TSS::Resetting TSS due to reason {} ", data.reason);
		} else {
			log::error!("TSS::unable to get reset reason");
		}
		self.tss_local_state.reset();
	}

	//Aggregator node signs the event and store it into local state
	pub async fn aggregator_event_sign(self: &mut Self, msg_hash: [u8; 64]) {
		let mut my_commitment = match self.tss_local_state.local_commitment_share.clone() {
			Some(commitment) => commitment,
			None => {
				log::error!("TSS::Unable to get local commitment share from local state");
				return;
			},
		};

		let final_state = match self.tss_local_state.local_finished_state.clone() {
			Some(final_state) => final_state,
			None => {
				log::error!("TSS::Unable to get local finished state from local state");
				return;
			},
		};

		if let Some(msg) = self.tss_local_state.msg_pool.get(&msg_hash) {
			//making partial signature here
			let partial_signature = match final_state.1.sign(
				&msg_hash,
				&final_state.0,
				&mut my_commitment.1,
				0,
				&self.tss_local_state.current_signers,
			) {
				Ok(partial_signature) => partial_signature,
				Err(e) => {
					log::error!("TSS::error occured while signing: {:?}", e);
					return;
				},
			};

			if let Some(hashmap) = self.tss_local_state.others_partial_signature.get_mut(&msg_hash)
			{
				hashmap.push(partial_signature);
			} else {
				let mut participant_list = Vec::new();
				participant_list.push(partial_signature);
				self.tss_local_state.others_partial_signature.insert(msg_hash, participant_list);
			}
			let message = match String::from_utf8(msg.clone()) {
				Ok(msg) => msg,
				Err(e) => {
					log::error!("TSS::error in converting message to string, {}", e);
					return;
				},
			};

			// FIXME: what's the point of this?
			match self.kv.sign(&self.kv.public_keys()[0], &msg) {
				Some(_) => {
					log::info!("message signed and stored successfully");
				},
				None => {
					log::error!("error in signing message");
				},
			};
		} else {
			log::error!("TSS::Message not found in pool");
		}
	}

	//Publishing the data to the network
	pub async fn publish_to_network<T>(
		self: &mut Self,
		peer_id: String,
		data: T,
		tss_type: TSSEventType,
	) where
		T: BorshSerialize,
	{
		log::info!("TSS::sending tss event: {:?}", tss_type);
		if let Ok(encoded_data) = data.try_to_vec() {
			if let Ok(data) = make_gossip_tss_data(peer_id, encoded_data, tss_type) {
				self.send(data);
			} else {
				log::error!("TSS::error making gossip data for encoded participant");
			}
		} else {
			//log error
			log::error!("TSS::tss error");
		}
	}
}
