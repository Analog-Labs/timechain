use crate::metadata::{self, runtime_types, RuntimeCall};
use crate::timechain_client::{
	BlockId, IBlock, IExtrinsic, ITimechainClient, ITransactionDbOps, ITransactionSubmitter,
};
use crate::ExtrinsicParams;
use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use futures::future::BoxFuture;
use futures::stream::{BoxStream, Fuse, FuturesUnordered};
use futures::{Future, FutureExt, StreamExt};
use scale_codec::{Decode, Encode};
use std::collections::{HashSet, VecDeque};
use std::pin::Pin;
use subxt::config::DefaultExtrinsicParamsBuilder;
use subxt::utils::H256;
use subxt_signer::sr25519::Keypair;
use time_primitives::BatchId;
use time_primitives::{
	traits::IdentifyAccount, AccountId, Commitment, GmpEvents, Network, NetworkConfig, NetworkId,
	PeerId, ProofOfKnowledge, PublicKey, ShardId, TaskId, TaskResult,
};

pub const MORTALITY: u8 = 32;
type TransactionFuture = Pin<Box<dyn Future<Output = Result<H256>> + Send>>;
type TransactionsUnordered = FuturesUnordered<TransactionFuture>;

#[derive(Clone, Encode, Decode)]
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
		// #[serde(with = "serde_proof_of_knowledge")]
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
	RestartBatch {
		batch_id: BatchId,
	},
}

#[derive(Clone, Encode, Decode)]
pub struct TxData {
	pub hash: H256,
	pub era: u64,
	pub nonce: u64,
	pub transaction: Tx,
}

pub struct TxStatus<C: ITimechainClient> {
	data: TxData,
	event_sender: Option<oneshot::Sender<<C::Block as IBlock>::Extrinsic>>,
	best_block: Option<u64>,
}

pub struct SubxtWorker<C, D>
where
	C: ITimechainClient + Send + Sync + Clone + 'static,
	D: ITransactionDbOps + Send + Sync + 'static,
{
	client: C,
	keypair: Keypair,
	nonce: u64,
	latest_block: BlockId,
	pending_tx: VecDeque<TxStatus<C>>,
	transaction_pool: TransactionsUnordered,
	db: D,
}

impl<C, D> SubxtWorker<C, D>
where
	C: ITimechainClient + Send + Sync + Clone + 'static,
	C::Submitter: ITransactionSubmitter + Send + Sync + 'static,
	C::Block: IBlock + Send + Sync + 'static,
	D: ITransactionDbOps + Send + Sync + 'static,
{
	pub async fn new(nonce: u64, client: C, db: D, keypair: Keypair) -> Result<Self> {
		let latest_block = client.get_latest_block().await?;
		let transaction_pool = FuturesUnordered::new();
		transaction_pool.push(futures::future::pending().boxed());

		let txs_data = db.load_pending_txs(nonce)?;
		let pending_tx: VecDeque<TxStatus<C>> = txs_data
			.into_iter()
			.map(|tx_data| TxStatus {
				data: tx_data,
				event_sender: None,
				best_block: None,
			})
			.collect();
		Ok(Self {
			client,
			keypair,
			nonce,
			latest_block,
			pending_tx,
			transaction_pool,
			db,
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
			Tx::RestartBatch { batch_id } => {
				let runtime_call = RuntimeCall::Tasks(
					metadata::runtime_types::pallet_tasks::pallet::Call::restart_batch { batch_id },
				);
				let payload = metadata::sudo(runtime_call);
				self.client.sign_payload(&payload, params)
			},
		}
	}

	fn add_tx_to_pool(
		&mut self,
		transaction: Tx,
		event_sender: Option<oneshot::Sender<<C::Block as IBlock>::Extrinsic>>,
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
			event_sender,
			best_block: None,
		};

		if let Err(e) = self.db.store_tx(&tx_status.data) {
			tracing::error!("Unable to add transaction into cache: {e}");
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
			let mut finalized_blocks = Self::create_stream_with_retry(self.client.clone(), |c| {
				async move { c.finalized_block_stream().await }.boxed()
			})
			.await;
			let mut best_blocks = Self::create_stream_with_retry(self.client.clone(), |c| {
				async move { c.best_block_stream().await }.boxed()
			})
			.await;
			loop {
				futures::select! {
					tx = rx.next().fuse() => {
						let Some((command, channel)) = tx else { break; };
						tracing::info!("tx added to pool");
						self.add_tx_to_pool(command, Some(channel), None);
					}
					block_data = finalized_blocks.next() => {
						match block_data {
							Some(Ok((_block, extrinsics))) => {
								if self.pending_tx.is_empty() {
									continue;
								}

								for extrinsic in extrinsics.into_iter() {
									let extrinsic_hash = extrinsic.hash();
									let front = self.pending_tx.front().map(|tx| tx.data.hash);
									if Some(extrinsic_hash) == front {
										let Some(tx) = self.pending_tx.pop_front() else {
											continue;
										};
										if let Err(e) = self.db.remove_tx(tx.data.hash){
											tracing::error!("Unable to remove tx from db {e}");
										};
										tx.event_sender.unwrap().send(extrinsic).ok();
									}
								}
							},
							Some(Err(e)) => {
								tracing::error!("Error processing finalized blocks: {:?}", e);
								tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
								continue;
							},
							None => {
								tracing::error!("Finalized block stream terminated");
								tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
								finalized_blocks = Self::
									create_stream_with_retry(
										self.client.clone(),
										|c| async move {
											let client = c.clone();
											client.finalized_block_stream().await
										}.boxed()
									)
									.await;
							}
						}
					}
					block_data = best_blocks.next() => {
						match block_data {
							Some(Ok((block, extrinsics))) => {
								self.latest_block = BlockId {
									number: block.number(),
									hash: block.hash()
								};
								tracing::info!("best block stream hit: {}", self.latest_block.number);

								if self.pending_tx.is_empty() {
									continue;
								}

								let hashes: HashSet<_> = extrinsics.iter()
									.map(|extrinsic| extrinsic.hash())
									.collect();
								for tx in self.pending_tx.iter_mut() {
									if hashes.contains(&tx.data.hash) {
										tracing::info!("tx found in block {}", self.latest_block.number);
										tx.best_block = Some(self.latest_block.number);
									}
								}

								let mut new_pending: VecDeque<TxStatus<C>> = VecDeque::new();
								while let Some(tx) = self.pending_tx.pop_front() {
									if tx.best_block.is_none() && self.latest_block.number > tx.data.era {
										tracing::warn!("outdated tx found retrying with nonce {}", tx.data.nonce);
										self.add_tx_to_pool(
											tx.data.transaction,
											tx.event_sender,
											Some(tx.data.nonce),
										);
									} else {
										new_pending.push_back(tx);
									}
								}
								self.pending_tx = new_pending;
							},
							Some(Err(e)) => {
								tracing::error!("Error processing finalized blocks: {:?}", e);
								tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
								continue;
							},
							None => {
								tracing::error!("Latest block stream terminated");
								tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
								best_blocks = Self::
									create_stream_with_retry(
										self.client.clone(),
										|c| async move {
											let client = c.clone();
											client.best_block_stream().await
										}.boxed()
									)
									.await;
							}
						}
					}
					tx_result = self.transaction_pool.next().fuse() => {
						let Some(result) = tx_result else {
							continue;
						};
						match result {
							Ok(hash) => {
								tracing::info!("tx completed: {:?}", hash);
							}
							Err(e) => {
								tracing::error!("tx failed {e}");
							}
						}
					}
					update = update_stream.next().fuse() => {
						let Some(Ok(update)) = update else {continue};
						self.client.apply_update(update).ok();
					}
				}
			}
		});
		tx
	}

	async fn create_stream_with_retry<S, F>(
		client: C,
		stream_creator: F,
	) -> Fuse<BoxStream<'static, Result<S>>>
	where
		F: Fn(C) -> BoxFuture<'static, Result<BoxStream<'static, Result<S>>>>
			+ Clone
			+ Send
			+ 'static,
	{
		loop {
			match stream_creator(client.clone()).await {
				Ok(stream) => {
					tracing::info!("stream created successfully returning");
					return stream.fuse();
				},
				Err(e) => {
					tracing::warn!("Couldn't create stream, retrying: {:?}", e);
					tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
				},
			}
		}
	}
}
