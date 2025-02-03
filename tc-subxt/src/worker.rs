use crate::metadata::{self, runtime_types, RuntimeCall};
use crate::{
	ExtrinsicDetails, ExtrinsicParams, LegacyRpcMethods, OnlineClient, SubmittableExtrinsic,
};
use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::stream::FuturesUnordered;
use futures::{Future, FutureExt, StreamExt, TryStreamExt};
use std::collections::{HashSet, VecDeque};
use std::pin::Pin;
use std::time::Duration;
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
type TransactionFuture = Pin<Box<dyn Future<Output = Result<H256>> + Send>>;
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

pub struct TxStatus<C: ITimechainClient> {
	data: TxData,
	event_sender: oneshot::Sender<<C::Block as IBlock>::Extrinsic>,
	best_block: Option<u64>,
}

pub struct SubxtWorker<C>
where
	C: ITimechainClient + Send + Sync + 'static,
{
	client: C,
	keypair: Keypair,
	nonce: u64,
	latest_block: BlockDetail,
	pending_tx: VecDeque<TxStatus<C>>,
	transaction_pool: TransactionsUnordered,
}

impl<C> SubxtWorker<C>
where
	C: ITimechainClient + Send + Sync + 'static,
	C::Submitter: ITransactionSubmitter + Send + Sync + 'static,
	C::Block: IBlock + Send + Sync + 'static,
{
	pub async fn new(nonce: u64, client: C, keypair: Keypair) -> Result<Self> {
		let block = client.get_latest_block().await?;
		let tx_pool = FuturesUnordered::new();
		// adding a never ending future to avoid tx_pool flood of None in select! loop
		tx_pool.push(futures::future::pending().boxed());
		Ok(Self {
			client,
			keypair,
			nonce,
			latest_block: block,
			pending_tx: Default::default(),
			transaction_pool: tx_pool,
		})
	}

	pub fn public_key(&self) -> PublicKey {
		PublicKey::Sr25519(self.keypair.public_key().as_ref().try_into().unwrap())
	}

	pub fn account_id(&self) -> AccountId {
		self.public_key().into_account()
	}

	fn build_tx(&mut self, tx: Tx, params: ExtrinsicParams) -> Vec<u8> {
		match tx.clone() {
			// system
			Tx::SetCode { code } => {
				let runtime_call =
					RuntimeCall::System(runtime_types::frame_system::pallet::Call::set_code {
						code,
					});
				let payload = metadata::sudo(runtime_call);
				self.client.sign_payload(&payload, params)
			},
			// balances
			Tx::Transfer { account, balance } => {
				let account = subxt::utils::Static(account);
				let payload =
					metadata::tx().balances().transfer_allow_death(account.into(), balance);
				self.client.sign_payload(&payload, params)
			},
			// networks
			Tx::RegisterNetwork { network } => {
				let network = subxt::utils::Static(network);
				let runtime_call = RuntimeCall::Networks(
					runtime_types::pallet_networks::pallet::Call::register_network { network },
				);
				let payload = metadata::sudo(runtime_call);
				self.client.sign_payload(&payload, params)
			},
			Tx::ForceShardOffline { shard_id } => {
				let runtime_call = RuntimeCall::Shards(
					metadata::runtime_types::pallet_shards::pallet::Call::force_shard_offline {
						shard_id,
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.client.sign_payload(&payload, params)
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
				self.client.sign_payload(&payload, params)
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
				self.client.sign_payload(&payload, params)
			},
			Tx::UnregisterMember { member } => {
				let member = subxt::utils::Static(member);
				let payload = metadata::tx().members().unregister_member(member);
				self.client.sign_payload(&payload, params)
			},
			Tx::Heartbeat => {
				let payload = metadata::tx().members().send_heartbeat();
				self.client.sign_payload(&payload, params)
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
				self.client.sign_payload(&payload, params)
			},
			Tx::Ready { shard_id } => {
				let payload = metadata::tx().shards().ready(shard_id);
				self.client.sign_payload(&payload, params)
			},
			// tasks
			Tx::SubmitTaskResult { task_id, result } => {
				let result = subxt::utils::Static(result);
				let payload = metadata::tx().tasks().submit_task_result(task_id, result);
				self.client.sign_payload(&payload, params)
			},
			Tx::SubmitGmpEvents { network, gmp_events } => {
				let runtime_call = RuntimeCall::Tasks(
					metadata::runtime_types::pallet_tasks::pallet::Call::submit_gmp_events {
						network,
						events: subxt::utils::Static(gmp_events),
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.client.sign_payload(&payload, params)
			},
			Tx::RemoveTask { task_id } => {
				let runtime_call = RuntimeCall::Tasks(
					metadata::runtime_types::pallet_tasks::pallet::Call::remove_task {
						task: task_id,
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.client.sign_payload(&payload, params)
			},
		}
	}

	fn add_tx_to_pool(
		&mut self,
		transaction: Tx,
		sender: oneshot::Sender<<C::Block as IBlock>::Extrinsic>,
		nonce: Option<u64>,
	) {
		let mut is_new_tx = true;
		let block = &self.latest_block;
		let nonce = match nonce {
			Some(nonce) => {
				is_new_tx = false;
				nonce
			},
			None => self.nonce,
		};
		let params: ExtrinsicParams = DefaultExtrinsicParamsBuilder::new()
			.nonce(nonce)
			.mortal_unchecked(block.number, block.hash, MORTALITY.into())
			.build();
		let tx = self.build_tx(transaction.clone(), params);
		let tx = self.client.submittable_transaction(tx.clone());
		let hash = tx.hash();
		let tx_status = TxStatus {
			data: TxData {
				transaction,
				era: self.latest_block.number + MORTALITY as u64,
				hash,
				nonce,
			},
			event_sender: sender,
			best_block: None,
		};
		if is_new_tx {
			self.pending_tx.push_back(tx_status);
			self.nonce += 1;
		} else {
			self.pending_tx.push_front(tx_status);
		}
		let fut = async move { tx.submit().await }.boxed();
		self.transaction_pool.push(fut);
	}

	pub fn into_sender(
		mut self,
	) -> mpsc::UnboundedSender<(Tx, oneshot::Sender<<C::Block as IBlock>::Extrinsic>)> {
		let (tx, mut rx) = mpsc::unbounded();
		tokio::task::spawn(async move {
			tracing::info!("starting subxt worker");
			let mut update_stream = self.client.runtime_updates().await.unwrap().boxed();
			let mut finalized_blocks = self.client.finalized_block_stream().await.unwrap().boxed();
			let mut best_blocks = self.client.best_block_stream().await.unwrap().boxed();
			loop {
				futures::select! {
					tx = rx.next().fuse() => {
						let Some((command, channel)) = tx else { continue; };
						self.add_tx_to_pool(command, channel, None);
					}
					block_data = finalized_blocks.next().fuse() => {
						let Some(block_res) = block_data else {
							tracing::error!("Finalized block strem terminated");
							tokio::time::sleep(Duration::from_secs(1)).await;
							continue;
						};

						let Ok((_, extrinsics)) = block_res else {
							tracing::error!("Error processing finalized blocks: {:?}", block_res.err());
							continue;
						};

						if self.pending_tx.is_empty() {
							continue;
						}

						for extrinsic in extrinsics.into_iter() {
							let extrinsic_hash = extrinsic.hash();
							if Some(extrinsic_hash) == self.pending_tx.front().map(|tx| tx.data.hash) {
								let Some(tx) = self.pending_tx.pop_front() else {
									continue;
								};
								tx.event_sender.send(extrinsic).ok();
							}
						}
					}
					block_data = best_blocks.next().fuse() => {
						let Some(block_res) = block_data else {
							tracing::error!("Latest block stream terminated");
							tokio::time::sleep(Duration::from_secs(1)).await;
							continue;
						};

						let Ok((block, extrinsics)) = block_res else {
							tracing::error!("Error processing block: {:?}", block_res.err());
							continue;
						};

						self.latest_block = BlockDetail {
							number: block.number().into(),
							hash: block.hash()
						};

						if self.pending_tx.is_empty() {
							continue;
						}

						let hashes: HashSet<_> = extrinsics.iter().map(|extrinsic| extrinsic.hash()).collect();
						for tx in self.pending_tx.iter_mut() {
							if hashes.contains(&tx.data.hash) {
								tx.best_block = Some(self.latest_block.number);
							}
						}

						let mut new_pending: VecDeque<TxStatus<C>> = VecDeque::new();
						while let Some(tx) = self.pending_tx.pop_front() {
							if tx.best_block.is_none() && self.latest_block.number > tx.data.era {
								tracing::warn!("Outdated tx found retrying with nonce: {}", tx.data.nonce);
								self.add_tx_to_pool(tx.data.transaction, tx.event_sender, Some(tx.data.nonce));
							} else {
								new_pending.push_back(tx);
							}
						}
						self.pending_tx = new_pending;
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
							}
						}
					}
					update = update_stream.next().fuse() => {
						let Some(Ok(update)) = update else {continue};
						self.client.apply_update(update);
					}
				}
			}
		});
		tx
	}
}
