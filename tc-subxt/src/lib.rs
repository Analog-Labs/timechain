use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use std::path::Path;
use std::str::FromStr;
use subxt::backend::rpc::RpcClient;
use subxt::tx::{SubmittableExtrinsic, TxPayload, TxStatus};
use subxt_signer::SecretUri;
use time_primitives::{
	AccountId, Balance, BlockHash, BlockNumber, Commitment, MemberStatus, NetworkId, PeerId,
	ProofOfKnowledge, PublicKey, Runtime, ShardId, ShardStatus, TaskDescriptor,
	TaskDescriptorParams, TaskExecution, TaskId, TaskResult, TssSignature,
};
use timechain_runtime::runtime_types::sp_runtime::MultiSigner as MetadataMultiSigner;
use timechain_runtime::runtime_types::time_primitives::task;
use timechain_runtime::runtime_types::timechain_runtime::RuntimeCall;
use tokio::sync::oneshot::{self, Sender};

mod shards;
mod tasks;

#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/metadata.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain_runtime {}

pub use subxt::backend::{
	rpc::{rpc_params, RpcParams},
	StreamOfResults,
};
pub use subxt::config::{Config, ExtrinsicParams};
pub use subxt::tx::PartialExtrinsic;
pub use subxt::utils::AccountId32;
pub use subxt::{ext, tx, utils};
pub use subxt::{OnlineClient, PolkadotConfig};
pub use subxt_signer::sr25519::Keypair;

pub type TxInBlock = subxt::tx::TxInBlock<PolkadotConfig, OnlineClient<PolkadotConfig>>;
pub type TxProgress = subxt::tx::TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>;

#[async_trait]
pub trait TxSubmitter: Clone + Send + Sync + 'static {
	async fn submit(&self, tx: Vec<u8>) -> Result<TxProgress>;
}

pub enum Tx {
	RegisterMember { network: NetworkId, peer_id: PeerId, stake_amount: u128 },
	Heartbeat,
	Commitment { shard_id: ShardId, commitment: Commitment, proof_of_knowledge: ProofOfKnowledge },
	CreateTask { task: TaskDescriptorParams },
	RegisterGateway { shard_id: ShardId, address: [u8; 20], block_height: u64 },
	SetShardConfig { shard_size: u16, shard_threshold: u16 },
	Ready { shard_id: ShardId },
	TaskHash { task_id: TaskId, hash: [u8; 32] },
	TaskResult { task_id: TaskId, result: TaskResult },
	TaskSignature { task_id: TaskId, signature: TssSignature },
}

struct SubxtWorker<T: TxSubmitter> {
	client: OnlineClient<PolkadotConfig>,
	keypair: Keypair,
	nonce: u64,
	tx_submitter: T,
}

impl<T: TxSubmitter> SubxtWorker<T> {
	pub async fn new(
		client: OnlineClient<PolkadotConfig>,
		keypair: Keypair,
		tx_submitter: T,
	) -> Result<Self> {
		let mut me = Self {
			client,
			keypair,
			nonce: 0,
			tx_submitter,
		};
		me.resync_nonce().await?;
		Ok(me)
	}

	fn public_key(&self) -> PublicKey {
		let public_key = self.keypair.public_key();
		PublicKey::Sr25519(unsafe { std::mem::transmute(public_key) })
	}

	fn account_id(&self) -> AccountId {
		let account_id: subxt::utils::AccountId32 = self.keypair.public_key().into();
		unsafe { std::mem::transmute(account_id) }
	}

	fn create_signed_payload<Call>(&self, call: &Call) -> Vec<u8>
	where
		Call: TxPayload,
	{
		self.client
			.tx()
			.create_signed_with_nonce(call, &self.keypair, self.nonce, Default::default())
			.unwrap()
			.into_encoded()
	}

	async fn resync_nonce(&mut self) -> Result<()> {
		let account_id: subxt::utils::AccountId32 = self.keypair.public_key().into();
		self.nonce = self.client.tx().account_nonce(&account_id).await?;
		Ok(())
	}

	pub async fn submit(&mut self, tx: (Tx, Sender<TxInBlock>)) {
		let (transaction, sender) = tx;
		let tx = match transaction {
			Tx::RegisterMember { network, peer_id, stake_amount } => {
				let public_key = self.public_key();
				let public_key: MetadataMultiSigner = unsafe { std::mem::transmute(public_key) };
				let tx = timechain_runtime::tx().members().register_member(
					network,
					public_key,
					peer_id,
					stake_amount,
				);
				self.create_signed_payload(&tx)
			},
			Tx::Heartbeat => {
				let tx = timechain_runtime::tx().members().send_heartbeat();
				self.create_signed_payload(&tx)
			},
			Tx::Commitment {
				shard_id,
				commitment,
				proof_of_knowledge,
			} => {
				let tx = timechain_runtime::tx().shards().commit(
					shard_id,
					commitment,
					proof_of_knowledge,
				);
				self.create_signed_payload(&tx)
			},
			Tx::Ready { shard_id } => {
				let tx = timechain_runtime::tx().shards().ready(shard_id);
				self.create_signed_payload(&tx)
			},
			Tx::TaskSignature { task_id, signature } => {
				let tx = timechain_runtime::tx().tasks().submit_signature(task_id, signature);
				self.create_signed_payload(&tx)
			},
			Tx::TaskHash { task_id, hash } => {
				let tx = timechain_runtime::tx().tasks().submit_hash(task_id, hash);
				self.create_signed_payload(&tx)
			},
			Tx::TaskResult { task_id, result } => {
				let result: task::TaskResult = unsafe { std::mem::transmute(result) };
				let tx = timechain_runtime::tx().tasks().submit_result(task_id, result);
				self.create_signed_payload(&tx)
			},
			Tx::CreateTask { task } => {
				let task_params: task::TaskDescriptorParams = unsafe { std::mem::transmute(task) };
				let tx = timechain_runtime::tx().tasks().create_task(task_params);
				self.create_signed_payload(&tx)
			},
			Tx::RegisterGateway {
				shard_id,
				address,
				block_height,
			} => {
				let runtime_call = RuntimeCall::Tasks(
					timechain_runtime::runtime_types::pallet_tasks::pallet::Call::register_gateway {
						bootstrap: shard_id,
						address,
						block_height,
					},
				);
				let sudo_call = timechain_runtime::tx().sudo().sudo(runtime_call);
				self.create_signed_payload(&sudo_call)
			},
			Tx::SetShardConfig { shard_size, shard_threshold } => {
				let runtime_call = RuntimeCall::Elections(timechain_runtime::runtime_types::pallet_elections::pallet::Call::set_shard_config {
					shard_size,
					shard_threshold,
				});
				let sudo_call = timechain_runtime::tx().sudo().sudo(runtime_call);
				self.create_signed_payload(&sudo_call)
			},
		};
		let result: Result<TxInBlock> = async {
			let mut tx_progress = self.tx_submitter.submit(tx).await?;
			while let Some(status) = tx_progress.next().await {
				match status? {
					// In block, return.
					TxStatus::InBestBlock(s) | TxStatus::InFinalizedBlock(s) => return Ok(s),
					// Error scenarios; return the error.
					TxStatus::Error { message } => {
						anyhow::bail!("tx error: {message}");
					},
					TxStatus::Invalid { message } => {
						anyhow::bail!("tx invalid: {message}");
					},
					TxStatus::Dropped { message } => {
						anyhow::bail!("tx dropped: {message}");
					},
					// Ignore and wait for next status event:
					_ => continue,
				}
			}
			anyhow::bail!("tx subscription dropped");
		}
		.await;

		match result {
			Ok(tx_in_block) => {
				sender.send(tx_in_block).ok();
				self.nonce += 1;
			},
			Err(err) => {
				tracing::error!("Error occured while submitting transaction: {err}");
				let nonce = self.nonce;
				if let Err(err) = self.resync_nonce().await {
					tracing::error!("failed to resync nonce: {err}");
				} else {
					tracing::info!("resynced nonce from {} to {}", nonce, self.nonce);
				}
			},
		}
	}

	fn into_sender(mut self) -> mpsc::UnboundedSender<(Tx, Sender<TxInBlock>)> {
		let updater = self.client.updater();
		let (tx, mut rx) = mpsc::unbounded();
		tokio::task::spawn(async move {
			tracing::info!("starting subxt worker");
			let mut update_stream =
				updater.runtime_updates().await.context("failed to start subxt worker").unwrap();
			loop {
				futures::select! {
					tx = rx.next().fuse() => {
						let Some(tx) = tx else { continue; };
						self.submit(tx).await;
					}
					update = update_stream.next().fuse() => {
						let Some(Ok(update)) = update else { continue; };
						let version = update.runtime_version().spec_version;
						match updater.apply_update(update) {
							Ok(()) => {
								tracing::info!("Upgrade to version: {} successful", version)
							},
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

#[derive(Clone)]
pub struct SubxtClient {
	client: OnlineClient<PolkadotConfig>,
	tx: mpsc::UnboundedSender<(Tx, Sender<TxInBlock>)>,
	public_key: PublicKey,
	account_id: AccountId,
}

impl SubxtClient {
	pub async fn new<T: TxSubmitter>(url: &str, keypair: Keypair, tx_submitter: T) -> Result<Self> {
		let client = Self::get_client(url).await?;
		let worker = SubxtWorker::new(client.clone(), keypair, tx_submitter).await?;
		let public_key = worker.public_key();
		let account_id = worker.account_id();
		tracing::info!("account id {}", account_id);
		let tx = worker.into_sender();
		Ok(Self {
			client,
			tx,
			public_key,
			account_id,
		})
	}

	pub async fn with_keyfile<T: TxSubmitter>(
		url: &str,
		keyfile: &Path,
		tx_submitter: T,
	) -> Result<Self> {
		let content = std::fs::read_to_string(keyfile)
			.context("failed to read substrate keyfile")
			.with_context(|| keyfile.display().to_string())?;
		let secret = SecretUri::from_str(&content).context("failed to parse substrate keyfile")?;
		let keypair =
			Keypair::from_uri(&secret).context("substrate keyfile contains invalid suri")?;
		Self::new(url, keypair, tx_submitter).await
	}

	pub async fn get_client(url: &str) -> Result<OnlineClient<PolkadotConfig>> {
		let rpc_client = RpcClient::from_url(url).await?;
		OnlineClient::<PolkadotConfig>::from_rpc_client(rpc_client.clone())
			.await
			.map_err(|_| anyhow::anyhow!("Failed to create a new client"))
	}

	pub async fn create_task(&self, task: TaskDescriptorParams) -> Result<TxInBlock> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::CreateTask { task }, tx))?;
		Ok(rx.await?)
	}

	pub async fn register_gateway(
		&self,
		shard_id: ShardId,
		address: [u8; 20],
		block_height: u64,
	) -> Result<TxInBlock> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((
			Tx::RegisterGateway {
				shard_id,
				address,
				block_height,
			},
			tx,
		))?;
		Ok(rx.await?)
	}

	pub async fn set_shard_config(
		&self,
		shard_size: u16,
		shard_threshold: u16,
	) -> Result<TxInBlock> {
		let (tx, rx) = oneshot::channel();
		self.tx
			.unbounded_send((Tx::SetShardConfig { shard_size, shard_threshold }, tx))?;
		Ok(rx.await?)
	}

	pub async fn get_latest_block(&self) -> Result<u64> {
		Ok(self.client.blocks().at_latest().await?.number().into())
	}
}

#[async_trait]
impl Runtime for SubxtClient {
	fn public_key(&self) -> &PublicKey {
		&self.public_key
	}

	fn account_id(&self) -> &AccountId {
		&self.account_id
	}

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		let api = self.client.clone();

		let stream = async_stream::stream! {
			let mut block_stream = match api.blocks().subscribe_finalized().await {
				Ok(stream) => {
					tracing::info!("got the stream finalized hit");
					stream
				},
				Err(e) => {
					tracing::error!("Error fetching block {:?}", e);
					yield Err(e);
					return;
				},
			};
			while let Some(block_result) = block_stream.next().await {
				match block_result {
					Ok(block) => {
						let block_hash = block.hash();
						let block_number = block.header().number;
						yield Ok((block_hash, block_number));
					},
					Err(e) => {
						tracing::error!("Error receiving block: {:?}", e);
						yield Err(e);
					},
				}
			}
		};

		let resolve_stream_values = stream.filter_map(|result| async move {
			match result {
				Ok(value) => Some(value),
				Err(e) => {
					tracing::error!("Error on finality stream {:?}", e);
					None
				},
			}
		});
		Box::pin(resolve_stream_values)
	}

	async fn get_network(&self, network: NetworkId) -> Result<Option<(String, String)>> {
		let runtime_call = timechain_runtime::apis().networks_api().get_network(network);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn get_member_peer_id(
		&self,
		_: BlockHash,
		account: &AccountId,
	) -> Result<Option<PeerId>> {
		let account: subxt::utils::AccountId32 = subxt::utils::AccountId32(*(account.as_ref()));
		let runtime_call = timechain_runtime::apis().members_api().get_member_peer_id(account);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn get_heartbeat_timeout(&self) -> Result<BlockNumber> {
		let runtime_call = timechain_runtime::apis().members_api().get_heartbeat_timeout();
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn get_min_stake(&self) -> Result<Balance> {
		let runtime_call = timechain_runtime::apis().members_api().get_min_stake();
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn get_shards(&self, _: BlockHash, account: &AccountId) -> Result<Vec<ShardId>> {
		let account: subxt::utils::AccountId32 = subxt::utils::AccountId32(*(account.as_ref()));
		let runtime_call = timechain_runtime::apis().shards_api().get_shards(account);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn get_shard_members(
		&self,
		_: BlockHash,
		shard_id: ShardId,
	) -> Result<Vec<(AccountId, MemberStatus)>> {
		let runtime_call = timechain_runtime::apis().shards_api().get_shard_members(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(unsafe { std::mem::transmute(data) })
	}

	async fn get_shard_threshold(&self, _: BlockHash, shard_id: ShardId) -> Result<u16> {
		let runtime_call = timechain_runtime::apis().shards_api().get_shard_threshold(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn get_shard_status(&self, _: BlockHash, shard_id: ShardId) -> Result<ShardStatus> {
		let runtime_call = timechain_runtime::apis().shards_api().get_shard_status(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(unsafe { std::mem::transmute(data) })
	}

	async fn get_shard_commitment(
		&self,
		_: BlockHash,
		shard_id: ShardId,
	) -> Result<Option<Commitment>> {
		let runtime_call = timechain_runtime::apis().shards_api().get_shard_commitment(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn get_shard_tasks(&self, _: BlockHash, shard_id: ShardId) -> Result<Vec<TaskExecution>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_shard_tasks(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(unsafe { std::mem::transmute(data) })
	}

	async fn get_task(&self, _: BlockHash, task_id: TaskId) -> Result<Option<TaskDescriptor>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_task(task_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(unsafe { std::mem::transmute(data) })
	}

	async fn get_task_signature(&self, task_id: TaskId) -> Result<Option<TssSignature>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_task_signature(task_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn get_task_signer(&self, task_id: TaskId) -> Result<Option<PublicKey>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_task_signer(task_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(unsafe { std::mem::transmute(data) })
	}

	async fn get_task_hash(&self, task_id: TaskId) -> Result<Option<[u8; 32]>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_task_hash(task_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn get_gateway(&self, network: NetworkId) -> Result<Option<[u8; 20]>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_gateway(network);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	async fn submit_register_member(
		&self,
		network: NetworkId,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx
			.unbounded_send((Tx::RegisterMember { network, peer_id, stake_amount }, tx))?;
		rx.await?;
		Ok(())
	}

	async fn submit_heartbeat(&self) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::Heartbeat, tx))?;
		rx.await?;
		Ok(())
	}

	async fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: [u8; 65],
	) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((
			Tx::Commitment {
				shard_id,
				commitment,
				proof_of_knowledge,
			},
			tx,
		))?;
		rx.await?;
		Ok(())
	}

	async fn submit_online(&self, shard_id: ShardId) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::Ready { shard_id }, tx))?;
		rx.await?;
		Ok(())
	}

	async fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::TaskSignature { task_id, signature }, tx))?;
		rx.await?;
		Ok(())
	}

	async fn submit_task_hash(&self, task_id: TaskId, hash: [u8; 32]) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::TaskHash { task_id, hash }, tx))?;
		rx.await?;
		Ok(())
	}

	async fn submit_task_result(&self, task_id: TaskId, result: TaskResult) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::TaskResult { task_id, result }, tx))?;
		rx.await?;
		Ok(())
	}
}

#[derive(Clone)]
pub struct SubxtTxSubmitter {
	client: OnlineClient<PolkadotConfig>,
}

impl SubxtTxSubmitter {
	pub async fn try_new(url: &str) -> Result<Self> {
		Ok(Self {
			client: SubxtClient::get_client(url).await?,
		})
	}
}

#[async_trait]
impl TxSubmitter for SubxtTxSubmitter {
	async fn submit(&self, tx: Vec<u8>) -> Result<TxProgress> {
		Ok(SubmittableExtrinsic::from_bytes(self.client.clone(), tx)
			.submit_and_watch()
			.await
			.map_err(|e| anyhow::anyhow!("Failed to Submit Tx {:?}", e))?)
	}
}
