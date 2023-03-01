use crate::{inherents::update_shared_group_key, traits::Client, worker::TimeWorker, TW_LOG};
use borsh::{BorshDeserialize, BorshSerialize};
use sc_client_api::Backend;
use sp_api::{BlockId, ProvideRuntimeApi};
use sp_blockchain::Backend as BCTrait;
use sp_consensus::SyncOracle;
use sp_runtime::traits::Block;
use std::collections::HashMap;
use time_primitives::TimeApi;
use tss::{local_state_struct::TSSLocalStateData, rand::rngs::OsRng};

use tss::{
	frost_dalek::{
		generate_commitment_share_lists, keygen::SecretShare, signature::ThresholdSignature,
		Participant, SignatureAggregator,
	},
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
	/// Initializes keygen and new state for given shard ID
	pub async fn handler_receive_params(&mut self, shard_id: u64, data: &[u8]) {
		if let Some(existing_state) = self.tss_local_states.get(&shard_id) {
			log::error!(
				target: TW_LOG,
				"Received params but node is not in empty state {:?}",
				existing_state.tss_process_state
			);
		} else {
			if let Some(local_peer_id) = self.id() {
				let mut state = self.create_tss_state();

				if let Ok(peer_id_call) = ReceiveParamsWithPeerCall::try_from_slice(data) {
					state.tss_params = peer_id_call.params;
					state.tss_process_state = TSSLocalStateType::ReceivedParams;

					let peer_id = peer_id_call.peer_id;
					if !state.others_peer_id.contains(&peer_id) {
						state.others_peer_id.push(peer_id);
					}

					if let Ok(peer_id_data) = get_publish_peer_id_msg(local_peer_id.clone()) {
						//nodes replies to this event with their peer id
						if let Ok(data) = make_gossip_tss_data(
							local_peer_id,
							peer_id_data,
							TSSEventType::ReceivePeerIDForIndex(shard_id),
						) {
							log::debug!("TSS: Sending ReceivePeerIDForIndex");
							self.send(data);
						} else {
							log::error!(
								target: TW_LOG,
								"Unable to encode gossip data for participant creation"
							);
						}
					} else {
						log::error!(target: TW_LOG, "Unable to get publish peer id msg");
					}
				} else {
					log::error!(target: TW_LOG, "Could not deserialize params");
				}
			} else {
				log::error!(
					target: TW_LOG,
					"No ID set for current node. Cant participate in TSS keygen"
				);
			}
		}
	}

	//used by node collector to set peers for tss process
	pub async fn handler_receive_peer_id_for_index(&mut self, shard_id: u64, data: &[u8]) {
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			let local_peer_id = state.local_peer_id.clone().unwrap();

			//receive index and update state of node
			if state.is_node_collector {
				if state.tss_process_state == TSSLocalStateType::Empty {
					if let Ok(peer_id_call) = PublishPeerIDCall::try_from_slice(data) {
						let peer_id = peer_id_call.peer_id;

						if !state.others_peer_id.contains(&peer_id) {
							state.others_peer_id.push(peer_id);

							let params = state.tss_params;

							//check if we have min number of nodes
							if state.others_peer_id.len() >= (params.n as usize - 1) {
								//change connector node state to received peers
								state.tss_process_state = TSSLocalStateType::ReceivedPeers;

								let mut other_peer_list = state.others_peer_id.clone();
								let index =
									get_participant_index(local_peer_id.clone(), &other_peer_list);
								state.local_index = Some(index);

								//collector node making participant and publishing
								let participant = make_participant(params, index);
								state.local_participant = Some(participant.clone());

								log::info!(
									target: TW_LOG,
									"this nodes participant index {}",
									index
								);

								//preparing publish data
								other_peer_list.push(state.local_peer_id.clone().unwrap());
								let data = FilterAndPublishParticipant {
									total_peer_list: other_peer_list,
									col_participant: participant.0,
								};

								//publish to network
								self.publish_to_network::<FilterAndPublishParticipant>(
									local_peer_id,
									data,
									TSSEventType::ReceivePeersWithColParticipant(shard_id),
								)
								.await;
							}
						}
					} else {
						log::error!(target: TW_LOG, "PeerID already exists in local state list");
					}
				} else {
					log::error!(
						target: TW_LOG,
						"Received peer id for index but node is not in empty state"
					);
				}
			}
		} else {
			log::debug!(target: TW_LOG, "TSS not started or not a member of shard: {}", shard_id);
		}
	}

	//filter participants and publish participants to network
	pub async fn handler_receiver_peers_with_col_participant(
		&mut self,
		shard_id: u64,
		data: &[u8],
	) {
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			let local_peer_id = state.local_peer_id.clone().unwrap();
			if state.tss_process_state == TSSLocalStateType::ReceivedParams {
				if let Ok(data) = FilterAndPublishParticipant::try_from_slice(data) {
					let mut other_peer_list = data.total_peer_list;

					if let Some(index) = other_peer_list.iter().position(|x| x.eq(&local_peer_id)) {
						other_peer_list.remove(index);
						state.tss_process_state = TSSLocalStateType::ReceivedPeers;
					} else {
						state.tss_process_state = TSSLocalStateType::NotParticipating;
						return;
					}

					if !state.others_participants.contains(&data.col_participant) {
						state.others_participants.push(data.col_participant);
					}

					state.others_peer_id = other_peer_list.clone();
					let index = get_participant_index(local_peer_id.clone(), &other_peer_list);
					state.local_index = Some(index);

					//make participant and publish
					let participant = make_participant(state.tss_params, index);
					state.local_participant = Some(participant.clone());

					self.publish_to_network(
						local_peer_id,
						participant.0,
						TSSEventType::ReceiveParticipant(shard_id),
					)
					.await;
				}
			} else {
				log::error!(
					target: TW_LOG,
					"Received peers with col participant but node is not in empty state"
				);
			}
		} else {
			log::debug!(
				target: TW_LOG,
				"Keygen not started or not a member of shard: {}",
				shard_id
			);
		}
	}

	//receive participant and publish to network secret share to network when all participants are
	// received
	pub async fn handler_receive_participant(&mut self, shard_id: u64, data: &[u8]) {
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			let local_peer_id = state.local_peer_id.clone().unwrap();
			//receive participants and update state of node
			if state.tss_process_state == TSSLocalStateType::ReceivedPeers {
				if let Ok(participant) = Participant::try_from_slice(data) {
					if !state.others_participants.contains(&participant) {
						state.others_participants.push(participant);
					}

					let params = state.tss_params;
					let total_nodes = params.n;
					let other_nodes = (state.others_participants.len() + 1) as u32;
					if total_nodes == other_nodes {
						// received total participants proceed to next process
						// creating secret share etc
						let local_index = match state.local_index {
							Some(index) => index,
							None => {
								log::error!(target: TW_LOG, "local index not found");
								return;
							},
						};
						let participant = match &state.local_participant.clone() {
							Some(participant) => participant.clone(),
							None => {
								log::error!(target: TW_LOG, "local participant not found");
								return;
							},
						};
						if let Ok(round_one_state) = round_one_state(
							&params,
							&local_index,
							&participant.1,
							&mut state.others_participants,
						) {
							//making hashmap of our their_secret_share
							let secret_shares = match round_one_state.their_secret_shares() {
								Ok(secret_shares) => secret_shares,
								Err(e) => {
									log::error!(
										target: TW_LOG,
										"error getting secret shares: {:#?}",
										e
									);
									return;
								},
							};

							//making hash table for secret share with index
							let distributed_hashmap =
								make_hashmap_for_secret_share(secret_shares).await;

							state.local_dkg_r1_state = Some(round_one_state.clone());

							state.tss_process_state = TSSLocalStateType::DkgGeneratedR1;
							log::info!(target: TW_LOG, "Keygen phase 1 done");

							//publish everyone's secret share to network
							self.publish_to_network::<HashMap<u32, SecretShare>>(
								local_peer_id,
								distributed_hashmap.clone(),
								TSSEventType::ReceiveSecretShare(shard_id),
							)
							.await;
						} else {
							log::error!(target: TW_LOG, "error in generating round one state");
						}
					}
				} else {
					//log error
					log::error!(target: TW_LOG, "Error deserializing participant");
				}
			} else {
				log::error!(
					target: TW_LOG,
					"Received participant but node is not in correct state: {:?}",
					state.tss_process_state
				);
			}
		} else {
			log::debug!(
				target: TW_LOG,
				"Keygen not started or not a member of shard: {}",
				shard_id
			);
		}
	}

	//receives secret share form all other nodes which are participating in the tss
	pub async fn handler_receive_secret_share(&mut self, shard_id: u64, data: &[u8]) {
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			let local_peer_id = state.local_peer_id.clone().unwrap();
			//receive secret shares and update state of node
			if state.tss_process_state == TSSLocalStateType::DkgGeneratedR1 {
				if let Ok(distributed_hashmap) = HashMap::<u32, SecretShare>::try_from_slice(data) {
					let local_index = match state.local_index {
						Some(index) => index,
						None => {
							log::error!(target: TW_LOG, "unable to get local index");
							return;
						},
					};
					if let Some(secret_share) = distributed_hashmap.get(&local_index) {
						if !state.others_my_secret_share.contains(secret_share) {
							state.others_my_secret_share.push(secret_share.clone());
						}

						let others_my_secret_shares = state.others_my_secret_share.clone();
						let params = state.tss_params;
						let total_nodes = params.n;
						let other_nodes = state.others_my_secret_share.len() as u32;
						if total_nodes == other_nodes + 1 {
							let round_one = match state.local_dkg_r1_state.clone() {
								Some(round_one) => round_one,
								None => {
									log::error!(
										target: TW_LOG,
										"Could not get round one state from local state"
									);
									return;
								},
							};
							match round_one.to_round_two(others_my_secret_shares) {
								Ok(round_two_state) => {
									state.local_dkg_r2_state = Some(round_two_state.clone());

									state.tss_process_state = TSSLocalStateType::DkgGeneratedR2;
									log::info!(target: TW_LOG, "Keygen phase 2 done");

									//finish local state progress
									let participant = match state.local_participant.clone() {
										Some(participant) => participant,
										None => {
											log::error!(
												target: TW_LOG,
												"Unable to get local participant from local state"
											);
											return;
										},
									};
									let my_commitment =
										match participant.0.public_key() {
											Some(commitment) => commitment,
											None => {
												log::error!(target: TW_LOG, "Unable to get commitment from local participant");
												return;
											},
										};
									if let Ok((local_group_key, local_secret_key)) =
										round_two_state.finish(my_commitment)
									{
										//update local state
										state.local_finished_state =
											Some((local_group_key, local_secret_key.clone()));
										state.local_public_key = Some(local_secret_key.to_public());
										state.tss_process_state = TSSLocalStateType::StateFinished;

										log::info!(target: TW_LOG, "==========================");
										let bytes = local_group_key.to_bytes();
										log::info!(
											target: TW_LOG,
											"local group key is: {:?}",
											bytes
										);
										update_shared_group_key(shard_id, bytes);
										log::info!(target: TW_LOG, "==========================");
									} else {
										log::error!(
											target: TW_LOG,
											"error occured while finishing state"
										);
									}

									//generating and publishing commitment to include node in tss
									// process
									let index = match state.local_index {
										Some(index) => index,
										None => {
											log::error!(
												target: TW_LOG,
												"unable to get local index"
											);
											return;
										},
									};
									if state.tss_process_state == TSSLocalStateType::StateFinished {
										//aggregator patch
										state.is_node_aggregator = state.is_node_collector;

										let local_commitment =
											generate_commitment_share_lists(&mut OsRng, index, 1);

										state.local_commitment_share =
											Some(local_commitment.clone());

										let pubkey = match state.local_public_key.clone() {
											Some(pubkey) => pubkey,
											None => {
												log::error!(
                                            target: TW_LOG, "Unable to get local public key from local state"
                                        );
												return;
											},
										};

										let share_commitment = OthersCommitmentShares {
											public_key: pubkey,
											public_commitment_share_list: local_commitment
												.0
												.clone(),
										};

										//publish publicCommitmentSharelist to network
										self.publish_to_network(
											local_peer_id,
											share_commitment,
											TSSEventType::ReceiveCommitment(shard_id),
										)
										.await;
									}
								},
								Err(e) => {
									log::error!(
										target: TW_LOG,
										"Error in round two state: {:#?}",
										e
									);
								},
							}
						} else {
							log::info!(target: TW_LOG, "Waiting for other nodes secret share");
						}
					} else {
						log::error!(target: TW_LOG, "Could not get secret share hash table");
					}
				} else {
					log::error!(target: TW_LOG, "Unable to deserialize secret share TSS Params");
				}
			} else {
				log::error!(
					target: TW_LOG,
					"Received secret share but node not in correct state {:?}",
					state.tss_process_state
				);
			}
		} else {
			log::debug!(
				target: TW_LOG,
				"Keygen not started or not a member of shard: {}",
				shard_id
			);
		}
	}

	//This call is received by aggregators to make list of commitment of participants
	pub async fn handler_receive_commitment(&mut self, shard_id: u64, data: &[u8]) {
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			//receive commitments and update state of node
			if state.is_node_aggregator {
				if state.tss_process_state >= TSSLocalStateType::DkgGeneratedR1
					&& state.tss_process_state <= TSSLocalStateType::StateFinished
				{
					if let Ok(commitment) = OthersCommitmentShares::try_from_slice(data) {
						if !state.others_commitment_share.contains(&commitment) {
							state.others_commitment_share.push(commitment);
						}

						let params = state.tss_params;
						if state.others_commitment_share.len() == (params.n - 1) as usize {
							log::info!(target: TW_LOG, "Received all commitments");
							state.tss_process_state = TSSLocalStateType::CommitmentsReceived;
						} else {
							log::info!(
								target: TW_LOG,
								"Not enough commitments, Got {}, Needed {}",
								state.others_commitment_share.len(),
								params.n - 1
							);
						}
					} else {
						log::error!(target: TW_LOG, "Unable to deserialize commitment");
					}
				} else {
					log::error!(
						target: TW_LOG,
						"Received commitment but node not in correct state"
					);
				}
			} else {
				log::info!(
					target: TW_LOG,
					"current tss state for receiving commitment is: {:?} with peer id: {:?}",
					state.tss_process_state,
					state.local_peer_id
				);
			}
		} else {
			log::debug!(
				target: TW_LOG,
				"Keygen not started or not a member of shard: {}",
				shard_id
			);
		}
	}

	//this call is received by aggregator to make the threshold signature
	pub async fn handler_partial_signature_received(&mut self, shard_id: u64, data: &[u8]) {
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			let local_peer_id = state.local_peer_id.clone().unwrap();

			//check if aggregator
			if state.is_node_aggregator {
				if state.tss_process_state == TSSLocalStateType::CommitmentsReceived {
					if let Ok(msg_req) = ReceivePartialSignatureReq::try_from_slice(data) {
						if let Some(msg) = state.msg_pool.get(&msg_req.msg_hash) {
							//add in list
							if let Some(hashmap) =
								state.others_partial_signature.get_mut(&msg_req.msg_hash)
							{
								hashmap.push(msg_req.partial_sign);
							} else {
								let participant_list = vec![msg_req.partial_sign];
								state
									.others_partial_signature
									.insert(msg_req.msg_hash, participant_list);
							}

							let params = state.tss_params;
							//the unwrap wont fail since we are already adding item above
							if state.others_partial_signature.get(&msg_req.msg_hash).unwrap().len()
								== state.current_signers.len()
							{
								let context = state.context;
								let finished_state = match state.local_finished_state.clone() {
									Some(finished_state) => finished_state,
									None => {
										log::error!(
											target: TW_LOG,
											"Unable to get local finished state from local state"
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

								for com in state.others_commitment_share.clone() {
									aggregator.include_signer(
										com.public_commitment_share_list.participant_index,
										com.public_commitment_share_list.commitments[0],
										com.public_key,
									);
								}

								aggregator.include_signer(
									state.local_index.unwrap(),
									state.local_commitment_share.clone().unwrap().0.commitments[0],
									state.local_public_key.clone().unwrap(),
								);

								//include partial signature
								for item in state
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
											log::error!(target: TW_LOG, "error occured while finalizing aggregator from index {:?} because of {:?}", key, value);
										}
										return;
									},
								};

								//aggregate aggregator
								let threshold_signature = match aggregator_finalized.aggregate() {
									Ok(threshold_signature) => threshold_signature,
									Err(e) => {
										for (key, value) in e.into_iter() {
											//can also send the indices of participants to
											// timechain to keep track of that
											log::error!(
												target: TW_LOG,
												"Participant {} misbehaved because {}",
												key,
												value
											);
										}
										return;
									},
								};

								//check for validity of event
								let data = match threshold_signature
									.verify(&finished_state.0, &msg_req.msg_hash)
								{
									Ok(_) => {
										log::info!(
											target: TW_LOG,
											"Signature is valid sending to network"
										);

										// workaround for absence of cloning
										let th_bytes = threshold_signature.to_bytes();
										// this can not fail
										let th = ThresholdSignature::from_bytes(th_bytes).unwrap();
										let gossip_data = VerifyThresholdSignatureReq {
											// msg: msg_req.msg,
											msg_hash: msg_req.msg_hash,
											threshold_sign: threshold_signature,
										};

										if state.is_node_aggregator {
											let key_bytes = th.to_bytes();
											let auth_key = self.kv.public_keys()[0].clone();
											let at =
												self.backend.blockchain().last_finalized().unwrap();
											let signature =
												self.kv.sign(&auth_key, &key_bytes).unwrap();
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
										Some(gossip_data)
									},
									Err(_) => {
										log::error!(
											target: TW_LOG,
											"Signature computed is invalid"
										);
										None
									},
								};

								//reset partial signature
								state.others_partial_signature.remove(&msg_req.msg_hash);

								//remove event from msg_pool
								state.msg_pool.remove(&msg_req.msg_hash);

								// publish back to network
								if let Some(data) = data {
									self.publish_to_network(
										local_peer_id,
										data,
										TSSEventType::VerifyThresholdSignature(shard_id),
									)
									.await;
								}
							}
						} else {
							log::error!(
								target: TW_LOG,
								"data received for signature but msg not in local pool {:?}",
								state.msg_pool.len()
							);
						}
					}
				} else {
					log::error!(
						target: TW_LOG,
						"Node not in correct state to receive partial signature"
					);
				}
			}
		} else {
			log::debug!(
				target: TW_LOG,
				"Keygen not started or not a member of shard: {}",
				shard_id
			);
		}
	}

	//This call is received by participant to verify the threshold signature
	pub async fn handler_verify_threshold_signature(&mut self, shard_id: u64, data: &[u8]) {
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			if state.tss_process_state >= TSSLocalStateType::StateFinished {
				if let Ok(threshold_signature) = VerifyThresholdSignatureReq::try_from_slice(data) {
					if state.msg_pool.get(&threshold_signature.msg_hash).is_some() {
						let finished_state = match state.local_finished_state.clone() {
							Some(finished_state) => finished_state,
							None => {
								log::error!(
									target: TW_LOG,
									"Unable to get local finished state from local state"
								);
								return;
							},
						};
						match threshold_signature
							.threshold_sign
							.verify(&finished_state.0, &threshold_signature.msg_hash)
						{
							Ok(_) => {
								//remove event from msg_pool
								state.msg_pool.remove(&threshold_signature.msg_hash);
								// signature should be stored by aggregator only
								// self.store_signature(threshold_signature.threshold_sign);
								log::info!("length of msg_pool {:?}", state.msg_pool.len());
							},
							Err(e) => {
								log::error!(target: TW_LOG, "Could not verify signature: {:?}", e);
							},
						}
					} else {
						log::error!(
							target: TW_LOG,
							"Could not find message in local pool for verification"
						);
					}
				} else {
					log::error!(
						target: TW_LOG,
						"Could not deserialize VerifiyThresholdSignatureReq"
					);
				}
			} else {
				log::error!(
					target: TW_LOG,
					"Node not in correct state to verify threshold signature, {:?}",
					state.tss_process_state
				);
			}
		} else {
			log::debug!(
				target: TW_LOG,
				"Keygen not started or not a member of shard: {}",
				shard_id
			);
		}
	}

	//This call resets the tss state data to empty/initial state
	pub async fn handler_reset_tss_state(&mut self, shard_id: u64, data: &[u8]) {
		//reset TSS State
		if let Ok(data) = ResetTSSCall::try_from_slice(data) {
			log::error!(target: TW_LOG, "Resetting TSS due to reason {} ", data.reason);
		} else {
			log::error!(target: TW_LOG, "unable to get reset reason");
		}
		// resetting only if state was created
		if let Some(state) = self.tss_local_states.get_mut(&shard_id) {
			state.reset();
		}
	}

	//Publishing the data to the network
	pub async fn publish_to_network<T>(&mut self, peer_id: String, data: T, tss_type: TSSEventType)
	where
		T: BorshSerialize,
	{
		log::info!(target: TW_LOG, "sending tss event: {:?}", tss_type);
		if let Ok(encoded_data) = data.try_to_vec() {
			if let Ok(data) = make_gossip_tss_data(peer_id, encoded_data, tss_type) {
				self.send(data);
			} else {
				log::error!(target: TW_LOG, "error making gossip data for encoded participant");
			}
		} else {
			//log error
			log::error!(target: TW_LOG, "tss error");
		}
	}
}

//Aggregator node signs the event and store it into local state
pub fn aggregator_event_sign(state: &mut TSSLocalStateData, msg_hash: [u8; 64]) {
	let mut my_commitment = match state.local_commitment_share.clone() {
		Some(commitment) => commitment,
		None => {
			log::error!(target: TW_LOG, "Unable to get local commitment share from local state");
			return;
		},
	};

	let final_state = match state.local_finished_state.clone() {
		Some(final_state) => final_state,
		None => {
			log::error!(target: TW_LOG, "Unable to get local finished state from local state");
			return;
		},
	};

	if state.msg_pool.contains_key(&msg_hash) {
		//making partial signature here
		let partial_signature = match final_state.1.sign(
			&msg_hash,
			&final_state.0,
			&mut my_commitment.1,
			0,
			&state.current_signers,
		) {
			Ok(partial_signature) => partial_signature,
			Err(e) => {
				log::error!(target: TW_LOG, "error occured while signing: {:?}", e);
				return;
			},
		};

		if let Some(hashmap) = state.others_partial_signature.get_mut(&msg_hash) {
			hashmap.push(partial_signature);
		} else {
			let participant_list = vec![partial_signature];
			state.others_partial_signature.insert(msg_hash, participant_list);
		}
	} else {
		log::error!(target: TW_LOG, "Message not found in pool");
	}
}

//This call is received by participant to generate its partial signature
pub fn handler_partial_signature_generate_req(
	state: &mut TSSLocalStateData,
	shard_id: u64,
	data: &[u8],
) -> Option<(String, ReceivePartialSignatureReq, TSSEventType)> {
	let local_peer_id = state.local_peer_id.clone().unwrap();

	if state.tss_process_state >= TSSLocalStateType::StateFinished {
		if let Ok(msg_req) = PartialMessageSign::try_from_slice(data) {
			let final_state = match state.local_finished_state.clone() {
				Some(final_state) => final_state,
				None => {
					log::error!(
						target: TW_LOG,
						"Unable to get local finished state from local state"
					);
					return None;
				},
			};

			let mut my_commitment = match state.local_commitment_share.clone() {
				Some(commitment) => commitment,
				None => {
					log::error!(
						target: TW_LOG,
						"Unable to get local commitment share from local state"
					);
					return None;
				},
			};

			if state.msg_pool.get(&msg_req.msg_hash).is_some() {
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
						log::error!(target: TW_LOG, "error occured while signing: {:?}", e);
						return None;
					},
				};

				let gossip_data = ReceivePartialSignatureReq {
					msg_hash: msg_req.msg_hash,
					partial_sign: partial_signature,
				};

				//publish partial signature to network
				return Some((
					local_peer_id,
					gossip_data,
					TSSEventType::PartialSignatureReceived(shard_id),
				));
			} else {
				log::warn!(
					target: TW_LOG,
					"data received for signing but not in local pool: {:?}",
					msg_req.msg_hash
				);
				state.msgs_signature_pending.insert(msg_req.msg_hash, msg_req.signers);
			}
		} else {
			log::error!(target: TW_LOG, "Unable to deserialize PartialMessageSign");
		}
	} else {
		log::error!(target: TW_LOG, "Node not in correct state to generate partial signature");
	}
	None
}
