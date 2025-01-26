use crate::metadata::{self, runtime_types, RuntimeCall};
use crate::{ExtrinsicEvents, LegacyRpcMethods, OnlineClient};
use std::collections::VecDeque;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, StreamExt};
use subxt::backend::rpc::RpcClient;
use subxt::config::DefaultExtrinsicParamsBuilder;
use subxt::tx::{Payload as TxPayload, SubmittableExtrinsic};
use subxt::utils::H256;
use subxt_signer::sr25519::Keypair;
use time_primitives::{
	traits::IdentifyAccount, AccountId, Commitment, GmpEvents, Network, NetworkConfig, NetworkId,
	PeerId, ProofOfKnowledge, PublicKey, ShardId, TaskId, TaskResult,
};
use tokio::sync::Mutex;

const MORTALITY: u8 = 32;

#[derive(Clone)]
pub enum Tx {
	// system
	SetCode {
		code: Vec<u8>,
	},
	// balances
	Transfer {
		account: AccountId,
		balance: u128,
	},
	// networks
	RegisterNetwork {
		network: Network,
	},
	ForceShardOffline {
		shard_id: ShardId,
	},
	SetNetworkConfig {
		network: NetworkId,
		config: NetworkConfig,
	},
	// members
	RegisterMember {
		network: NetworkId,
		public_key: PublicKey,
		peer_id: PeerId,
		stake_amount: u128,
	},
	UnregisterMember {
		member: AccountId,
	},
	Heartbeat,
	// shards
	Commitment {
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	},
	Ready {
		shard_id: ShardId,
	},
	// tasks
	SubmitTaskResult {
		task_id: TaskId,
		result: TaskResult,
	},
	SubmitGmpEvents {
		network: NetworkId,
		gmp_events: GmpEvents,
	},
	RemoveTask {
		task_id: TaskId,
	},
}

pub struct TxStatus {
	hash: H256,
	era: u64,
	event_sender: oneshot::Sender<ExtrinsicEvents>,
	data: Tx,
	nonce: u64,
	best_block: Option<u64>,
}

pub struct BlockDetail {
	number: u64,
	hash: H256,
}

pub struct SubxtWorker {
	rpc: RpcClient,
	client: OnlineClient,
	keypair: Keypair,
	nonce: u64,
	latest_block: Option<BlockDetail>,
	pending_tx: Arc<Mutex<VecDeque<TxStatus>>>,
}

impl SubxtWorker {
	pub async fn new(rpc: RpcClient, client: OnlineClient, keypair: Keypair) -> Result<Self> {
		let mut me = Self {
			rpc,
			client,
			keypair,
			nonce: 0,
			latest_block: None,
			pending_tx: Default::default(),
		};
		me.resync_nonce().await?;
		Ok(me)
	}

	pub fn public_key(&self) -> PublicKey {
		PublicKey::Sr25519(self.keypair.public_key().as_ref().try_into().unwrap())
	}

	pub fn account_id(&self) -> AccountId {
		self.public_key().into_account()
	}

	async fn create_signed_payload<Call>(&self, call: &Call, nonce: Option<u64>) -> (u64, Vec<u8>)
	where
		Call: TxPayload,
	{
		let (block_number, block_hash) = match &self.latest_block {
			Some(block) => (block.number, block.hash),
			None => {
				let block = self.client.blocks().at_latest().await.unwrap();
				(block.number().into(), block.hash())
			},
		};
		let nonce = match nonce {
			Some(nonce) => nonce,
			None => self.nonce,
		};
		let params = DefaultExtrinsicParamsBuilder::new()
			.nonce(nonce)
			.mortal_unchecked(block_number, block_hash, MORTALITY.into())
			.build();
		let tx = self
			.client
			.tx()
			.create_signed(call, &self.keypair, params)
			.await
			.unwrap()
			.into_encoded();
		(block_number + MORTALITY as u64, tx)
	}

	async fn resync_nonce(&mut self) -> Result<()> {
		let account_id: subxt::utils::AccountId32 = self.keypair.public_key().into();
		let rpc = LegacyRpcMethods::new(self.rpc.clone());
		self.nonce = rpc.system_account_next_index(&account_id).await?;
		Ok(())
	}

	pub async fn submit(&mut self, tx: (Tx, oneshot::Sender<ExtrinsicEvents>), nonce: Option<u64>) {
		let (transaction, sender) = tx;
		let (era, tx) = match transaction.clone() {
			// system
			Tx::SetCode { code } => {
				let runtime_call =
					RuntimeCall::System(runtime_types::frame_system::pallet::Call::set_code {
						code,
					});
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce).await
			},
			// balances
			Tx::Transfer { account, balance } => {
				let account = subxt::utils::Static(account);
				let payload =
					metadata::tx().balances().transfer_allow_death(account.into(), balance);
				self.create_signed_payload(&payload, nonce).await
			},
			// networks
			Tx::RegisterNetwork { network } => {
				let network = subxt::utils::Static(network);
				let runtime_call = RuntimeCall::Networks(
					runtime_types::pallet_networks::pallet::Call::register_network { network },
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce).await
			},
			Tx::ForceShardOffline { shard_id } => {
				let runtime_call = RuntimeCall::Shards(
					metadata::runtime_types::pallet_shards::pallet::Call::force_shard_offline {
						shard_id,
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce).await
			},
			Tx::SetNetworkConfig { network, config } => {
				let config = subxt::utils::Static(config);
				let runtime_call = RuntimeCall::Networks(
					metadata::runtime_types::pallet_networks::pallet::Call::set_network_config {
						network,
						config,
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce).await
			},
			// members
			Tx::RegisterMember {
				network,
				public_key,
				peer_id,
				stake_amount,
			} => {
				let public_key = subxt::utils::Static(public_key);
				let payload = metadata::tx().members().register_member(
					network,
					public_key,
					peer_id,
					stake_amount,
				);
				self.create_signed_payload(&payload, nonce).await
			},
			Tx::UnregisterMember { member } => {
				let member = subxt::utils::Static(member);
				let payload = metadata::tx().members().unregister_member(member);
				self.create_signed_payload(&payload, nonce).await
			},
			Tx::Heartbeat => {
				let payload = metadata::tx().members().send_heartbeat();
				self.create_signed_payload(&payload, nonce).await
			},
			// shards
			Tx::Commitment {
				shard_id,
				commitment,
				proof_of_knowledge,
			} => {
				let commitment = subxt::utils::Static(commitment);
				let payload =
					metadata::tx().shards().commit(shard_id, commitment, proof_of_knowledge);
				self.create_signed_payload(&payload, nonce).await
			},
			Tx::Ready { shard_id } => {
				let payload = metadata::tx().shards().ready(shard_id);
				self.create_signed_payload(&payload, nonce).await
			},
			// tasks
			Tx::SubmitTaskResult { task_id, result } => {
				let result = subxt::utils::Static(result);
				let payload = metadata::tx().tasks().submit_task_result(task_id, result);
				self.create_signed_payload(&payload, nonce).await
			},
			Tx::SubmitGmpEvents { network, gmp_events } => {
				let runtime_call = RuntimeCall::Tasks(
					metadata::runtime_types::pallet_tasks::pallet::Call::submit_gmp_events {
						network,
						events: subxt::utils::Static(gmp_events),
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce).await
			},
			Tx::RemoveTask { task_id } => {
				let runtime_call = RuntimeCall::Tasks(
					metadata::runtime_types::pallet_tasks::pallet::Call::remove_task {
						task: task_id,
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce).await
			},
		};

		let tx_hash = SubmittableExtrinsic::from_bytes(self.client.clone(), tx).submit().await;
		match tx_hash {
			Ok(tx_hash) => {
				let mut pending_tx = self.pending_tx.lock().await;
				let tx_status = TxStatus {
					hash: tx_hash,
					era,
					event_sender: sender,
					data: transaction,
					nonce: self.nonce,
					best_block: None,
				};
				if nonce.is_some() {
					// if nonce is received that means its a retry due to mortality outage
					pending_tx.push_front(tx_status);
				} else {
					pending_tx.push_back(tx_status);
					self.nonce += 1;
				}
			},
			Err(e) => {
				tracing::error!("Error submitting transaction to timechain {e}");
				let nonce = self.nonce;
				if let Err(err) = self.resync_nonce().await {
					tracing::error!("failed to resync nonce: {err}");
				} else {
					tracing::info!("resynced nonce from {} to {}", nonce, self.nonce);
				}
			},
		}
	}

	async fn complete_received_txs(&mut self, finalized_block: crate::Block) -> Result<()> {
		let mut pending_tx = self.pending_tx.lock().await;
		if pending_tx.is_empty() {
			return Ok(());
		}
		for extrinsic in finalized_block.extrinsics().await?.iter() {
			let extrinsic_hash = extrinsic.hash();
			if Some(extrinsic_hash) == pending_tx.front().map(|tx| tx.hash) {
				let extrinsic_events = extrinsic.events().await?;
				let tx = pending_tx.pop_front().unwrap();
				tx.event_sender.send(extrinsic_events).ok();
			}
		}
		Ok(())
	}

	async fn check_outdated_txs(&mut self, best_block: crate::Block) -> Result<()> {
		let mut pending_tx = self.pending_tx.lock().await;
		if pending_tx.is_empty() {
			return Ok(());
		}

		let extrinsic_hashes: Vec<_> = best_block
			.extrinsics()
			.await?
			.iter()
			.map(|extrinsic| extrinsic.hash())
			.collect();

		let block_number: u64 = best_block.number().into();

		for tx in pending_tx.iter_mut() {
			if extrinsic_hashes.contains(&tx.hash) {
				tx.best_block = Some(block_number);
			}
		}

		let mut outdated_txs = vec![];
		let mut i = 0;
		while i < pending_tx.len() {
			if pending_tx[i].best_block.is_none() && block_number > pending_tx[i].era {
				let Some(tx_data) = pending_tx.remove(i) else {
					continue;
				};
				outdated_txs.push(tx_data);
			} else {
				i += 1;
			}
		}

		drop(pending_tx);
		for tx in outdated_txs {
			self.submit((tx.data, tx.event_sender), Some(tx.nonce)).await;
		}
		Ok(())
	}

	pub fn into_sender(mut self) -> mpsc::UnboundedSender<(Tx, oneshot::Sender<ExtrinsicEvents>)> {
		let updater = self.client.updater();
		let (tx, mut rx) = mpsc::unbounded();
		tokio::task::spawn(async move {
			tracing::info!("starting subxt worker");
			let mut update_stream =
				updater.runtime_updates().await.context("failed to start subxt worker").unwrap();
			let mut finalized_block_stream =
				self.client.blocks().subscribe_finalized().await.unwrap();
			let mut best_block_stream = self.client.blocks().subscribe_best().await.unwrap();
			loop {
				futures::select! {
					tx = rx.next().fuse() => {
						let Some(tx) = tx else { continue; };
						self.submit(tx, None).await;
					}
					finalized_block = finalized_block_stream.next().fuse() => {
						if let Some(finalized_block) = finalized_block {
							let block = finalized_block.unwrap();
							if let Err(e) = self.complete_received_txs(block).await{
								tracing::error!("Error completing transaction {e}");
							};
						}
					}
					best_block = best_block_stream.next().fuse() => {
						if let Some(best_block) = best_block {
							let block = best_block.unwrap();
							self.latest_block = Some(BlockDetail {
								number: block.number().into(),
								hash: block.hash()
							});
							if let Err(e) = self.check_outdated_txs(block).await {
								tracing::error!("Error while retrying transactions: {e}");
							};
						}
					}
					update = update_stream.next().fuse() => {
						let Some(Ok(update)) = update else { continue; };
						let version = update.runtime_version().spec_version;
						match updater.apply_update(update) {
							Ok(()) => {
								tracing::info!("Upgrade to version: {} successful", version)
							},
							Err(subxt::client::UpgradeError::SameVersion) => {}
							Err(e) => {
								tracing::error!("Upgrade to version {} failed {:?}", version, e);
							},
						};
					}
				}
			}
		});
		tx
	}
}
