use crate::metadata::{self, runtime_types, RuntimeCall};
use crate::{ExtrinsicDetails, LegacyRpcMethods, OnlineClient, SubmittableExtrinsic};
use std::collections::VecDeque;
use std::pin::Pin;

use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::stream::FuturesUnordered;
use futures::{Future, FutureExt, StreamExt};
use subxt::backend::rpc::RpcClient;
use subxt::config::DefaultExtrinsicParamsBuilder;
use subxt::tx::Payload as TxPayload;
use subxt::utils::H256;
use subxt_signer::sr25519::Keypair;
use time_primitives::{
	traits::IdentifyAccount, AccountId, Commitment, GmpEvents, Network, NetworkConfig, NetworkId,
	PeerId, ProofOfKnowledge, PublicKey, ShardId, TaskId, TaskResult,
};

const MORTALITY: u8 = 32;
type TransactionFuture = Pin<Box<dyn Future<Output = Result<H256, subxt::Error>> + Send>>;
type TransactionsUnordered = FuturesUnordered<TransactionFuture>;

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

pub struct TxData {
	transaction: Tx,
	era: u64,
	hash: H256,
	nonce: u64,
}

pub struct TxStatus {
	data: TxData,
	event_sender: oneshot::Sender<ExtrinsicDetails>,
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
	latest_block: BlockDetail,
	pending_tx: VecDeque<TxStatus>,
	transaction_pool: TransactionsUnordered,
}

impl SubxtWorker {
	pub async fn new(rpc: RpcClient, client: OnlineClient, keypair: Keypair) -> Result<Self> {
		let block = client.blocks().at_latest().await?;
		let mut me = Self {
			rpc,
			client,
			keypair,
			nonce: 0,
			latest_block: BlockDetail {
				number: block.number().into(),
				hash: block.hash(),
			},
			pending_tx: Default::default(),
			transaction_pool: FuturesUnordered::new(),
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

	fn create_signed_payload<Call>(&self, call: &Call, nonce: Option<u64>) -> (u64, Vec<u8>)
	where
		Call: TxPayload,
	{
		let block = &self.latest_block;
		let nonce = match nonce {
			Some(nonce) => nonce,
			None => self.nonce,
		};
		let params = DefaultExtrinsicParamsBuilder::new()
			.nonce(nonce)
			.mortal_unchecked(block.number, block.hash, MORTALITY.into())
			.build();
		let tx = self
			.client
			.tx()
			.create_signed_offline(call, &self.keypair, params)
			.expect("Metadata is invalid")
			.into_encoded();
		(block.number + MORTALITY as u64, tx)
	}

	async fn resync_nonce(&mut self) -> Result<()> {
		let account_id: subxt::utils::AccountId32 = self.keypair.public_key().into();
		let rpc = LegacyRpcMethods::new(self.rpc.clone());
		self.nonce = rpc.system_account_next_index(&account_id).await?;
		Ok(())
	}

	fn submit(&mut self, tx: (Tx, oneshot::Sender<ExtrinsicDetails>), nonce: Option<u64>) {
		let (transaction, sender) = tx;
		let (era, tx) = match transaction.clone() {
			// system
			Tx::SetCode { code } => {
				let runtime_call =
					RuntimeCall::System(runtime_types::frame_system::pallet::Call::set_code {
						code,
					});
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce)
			},
			// balances
			Tx::Transfer { account, balance } => {
				let account = subxt::utils::Static(account);
				let payload =
					metadata::tx().balances().transfer_allow_death(account.into(), balance);
				self.create_signed_payload(&payload, nonce)
			},
			// networks
			Tx::RegisterNetwork { network } => {
				let network = subxt::utils::Static(network);
				let runtime_call = RuntimeCall::Networks(
					runtime_types::pallet_networks::pallet::Call::register_network { network },
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce)
			},
			Tx::ForceShardOffline { shard_id } => {
				let runtime_call = RuntimeCall::Shards(
					metadata::runtime_types::pallet_shards::pallet::Call::force_shard_offline {
						shard_id,
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce)
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
				self.create_signed_payload(&payload, nonce)
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
				self.create_signed_payload(&payload, nonce)
			},
			Tx::UnregisterMember { member } => {
				let member = subxt::utils::Static(member);
				let payload = metadata::tx().members().unregister_member(member);
				self.create_signed_payload(&payload, nonce)
			},
			Tx::Heartbeat => {
				let payload = metadata::tx().members().send_heartbeat();
				self.create_signed_payload(&payload, nonce)
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
				self.create_signed_payload(&payload, nonce)
			},
			Tx::Ready { shard_id } => {
				let payload = metadata::tx().shards().ready(shard_id);
				self.create_signed_payload(&payload, nonce)
			},
			// tasks
			Tx::SubmitTaskResult { task_id, result } => {
				let result = subxt::utils::Static(result);
				let payload = metadata::tx().tasks().submit_task_result(task_id, result);
				self.create_signed_payload(&payload, nonce)
			},
			Tx::SubmitGmpEvents { network, gmp_events } => {
				let runtime_call = RuntimeCall::Tasks(
					metadata::runtime_types::pallet_tasks::pallet::Call::submit_gmp_events {
						network,
						events: subxt::utils::Static(gmp_events),
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce)
			},
			Tx::RemoveTask { task_id } => {
				let runtime_call = RuntimeCall::Tasks(
					metadata::runtime_types::pallet_tasks::pallet::Call::remove_task {
						task: task_id,
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload, nonce)
			},
		};

		let tx = SubmittableExtrinsic::from_bytes(self.client.clone(), tx);
		let hash = tx.hash();
		let tx_status = TxStatus {
			data: TxData {
				transaction,
				era,
				hash,
				nonce: self.nonce,
			},
			event_sender: sender,
			best_block: None,
		};
		if nonce.is_some() {
			// if nonce is received that means its a retry due to mortality outage
			self.pending_tx.push_front(tx_status);
		} else {
			self.pending_tx.push_back(tx_status);
			self.nonce += 1;
		}
		let fut = async move { tx.submit().await }.boxed();
		self.transaction_pool.push(fut);
	}

	fn complete_received_txs(&mut self, extrinsics: Vec<ExtrinsicDetails>) {
		if self.pending_tx.is_empty() {
			return;
		}
		for extrinsic in extrinsics {
			let extrinsic_hash = extrinsic.hash();
			if Some(extrinsic_hash) == self.pending_tx.front().map(|tx| tx.data.hash) {
				let tx = self
					.pending_tx
					.pop_front()
					.ok_or_else(|| anyhow::anyhow!("Failed to pull tx from pending stack"))?;
				tx.event_sender.send(extrinsic).ok();
			}
		}
	}

	fn check_outdated_txs(&mut self, block: u64, extrinsics: Vec<H256>) {
		if self.pending_tx.is_empty() {
			return;
		}

		for tx in self.pending_tx.iter_mut() {
			if extrinsics.contains(&tx.data.hash) {
				tx.best_block = Some(block);
			}
		}

		let mut i = 0;
		while i < self.pending_tx.len() {
			if self.pending_tx[i].best_block.is_none() && block > self.pending_tx[i].data.era {
				let Some(tx) = self.pending_tx.remove(i) else {
					tracing::warn!(
						"Outdated transaction found, but removal from cache failed: index: {i} len: {}", self.pending_tx.len()
					);
					continue;
				};
				self.submit((tx.data.transaction, tx.event_sender), Some(tx.data.nonce));
			} else {
				i += 1;
			}
		}
	}

	pub fn into_sender(mut self) -> mpsc::UnboundedSender<(Tx, oneshot::Sender<ExtrinsicDetails>)> {
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
						self.submit(tx, None);
					}
					finalized_block = finalized_block_stream.next().fuse() => {
						if let Some(finalized_block) = finalized_block {
							let Ok(block) = finalized_block else {
								tracing::info!("Error while getting finalized block");
								continue;
							};
							let Ok(extrinsics) = block.extrinsics().await else {
								tracing::error!("Block Extrinsics not found");
								continue;
							};
							let extrinsics = extrinsics.iter().collect::<Vec<_>>();
							self.complete_received_txs(extrinsics);
						}
					}
					best_block = best_block_stream.next().fuse() => {
						if let Some(Ok(block)) = best_block {
							self.latest_block = BlockDetail {
								number: block.number().into(),
								hash: block.hash()
							};
							let Ok(extrinsics) = block.extrinsics().await else {
								tracing::error!("Block Extrinsics not found");
								continue;
							};
							let hashes: Vec<_> = extrinsics.iter().map(|extrinsic| extrinsic.hash()).collect();
							self.check_outdated_txs(block.number().into(), hashes);
						}
					}
					tx_result = self.transaction_pool.next().fuse() => {
						let Some(result) = tx_result else {
							continue;
						};
						match result {
							Ok(hash) => {
								tracing::info!("Transaction completed: {:?}", hash);
							}
							Err(e) => {
								tracing::error!("Transaction failed {e}");
								let nonce = self.nonce;
								if let Err(err) = self.resync_nonce().await {
									tracing::error!("failed to resync nonce: {err}");
								} else {
									tracing::info!("resynced nonce from {} to {}", nonce, self.nonce);
								}
							}
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
