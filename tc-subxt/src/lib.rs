use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::stream::{self, BoxStream};
use futures::StreamExt;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use subxt::backend::rpc::RpcClient;
use subxt::blocks::ExtrinsicEvents;
use subxt::dynamic::Value;
use subxt::ext::scale_value::Primitive;
use subxt::tx::SubmittableExtrinsic;
use subxt::tx::TxPayload;
use subxt::utils::{MultiAddress, MultiSignature, H256};
use subxt_signer::SecretUri;
use time_primitives::{
	AccountId, BlockHash, BlockNumber, Commitment, MemberStatus, NetworkId, PeerId,
	ProofOfKnowledge, PublicKey, Runtime, ShardId, ShardStatus, TaskCycle, TaskDescriptor,
	TaskError, TaskExecution, TaskId, TaskResult, TssPublicKey, TssSignature,
};
use timechain_runtime::runtime_types::sp_runtime::MultiSigner as MetadataMultiSigner;
use timechain_runtime::runtime_types::time_primitives::task;
use timechain_runtime::runtime_types::timechain_runtime::RuntimeCall;

mod shards;
mod tasks;

#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/metadata.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain_runtime {}

pub use subxt::backend::rpc::{rpc_params, RpcParams};
pub use subxt::config::{Config, ExtrinsicParams};
pub use subxt::tx::PartialExtrinsic;
pub use subxt::utils::AccountId32;
pub use subxt::{ext, tx, utils};
pub use subxt::{OnlineClient, PolkadotConfig};
pub use subxt_signer::sr25519::Keypair;

enum Tx {
	RegisterMember { network: NetworkId, peer_id: PeerId, stake_amount: u128 },
	Heartbeat,
	Commitment { shard_id: ShardId, commitment: Commitment, proof_of_knowledge: [u8; 65] },
	Ready { shard_id: ShardId },
	TaskHash { task_id: TaskId, cycle: TaskCycle, hash: Vec<u8> },
	TaskResult { task_id: TaskId, cycle: TaskCycle, result: TaskResult },
	TaskError { task_id: TaskId, cycle: TaskCycle, error: TaskError },
	TaskSignature { task_id: TaskId, signature: TssSignature },
}

struct SubxtWorker {
	client: OnlineClient<PolkadotConfig>,
	keypair: Keypair,
	nonce: u64,
}

impl SubxtWorker {
	pub async fn new(client: OnlineClient<PolkadotConfig>, keypair: Keypair) -> Result<Self> {
		let account_id: subxt::utils::AccountId32 = keypair.public_key().into();
		let nonce = client.tx().account_nonce(&account_id).await?;
		Ok(Self { client, keypair, nonce })
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

	pub async fn submit(&mut self, tx: Tx) -> Result<H256> {
		let tx = match tx {
			Tx::RegisterMember { network, peer_id, stake_amount } => {
				let public_key = self.public_key();
				let network = unsafe { std::mem::transmute(network) };
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
			Tx::TaskHash { task_id, cycle, hash } => {
				let tx = timechain_runtime::tx().tasks().submit_hash(task_id, cycle, hash);
				self.create_signed_payload(&tx)
			},
			Tx::TaskResult { task_id, cycle, result } => {
				let result: task::TaskResult = unsafe { std::mem::transmute(result) };
				let tx = timechain_runtime::tx().tasks().submit_result(task_id, cycle, result);
				self.create_signed_payload(&tx)
			},
			Tx::TaskError { task_id, cycle, error } => {
				let error: task::TaskError = unsafe { std::mem::transmute(error) };
				let tx = timechain_runtime::tx().tasks().submit_error(task_id, cycle, error);
				self.create_signed_payload(&tx)
			},
		};
		let hash = SubmittableExtrinsic::from_bytes(self.client.clone(), tx).submit().await?;
		self.nonce += 1;
		Ok(hash)
	}

	fn into_sender(mut self) -> mpsc::UnboundedSender<Tx> {
		let (tx, mut rx) = mpsc::unbounded();
		tokio::task::spawn(async move {
			while let Some(tx) = rx.next().await {
				if let Err(err) = self.submit(tx).await {
					tracing::error!("{err}");
				}
			}
		});
		tx
	}
}

#[derive(Clone)]
pub struct SubxtClient {
	client: OnlineClient<PolkadotConfig>,
	tx: mpsc::UnboundedSender<Tx>,
}

impl SubxtClient {
	pub async fn new(url: &str, keypair: Keypair) -> Result<Self> {
		let rpc_client = RpcClient::from_url(url).await?;
		let client = OnlineClient::<PolkadotConfig>::from_rpc_client(rpc_client.clone()).await?;
		let tx = SubxtWorker::new(client.clone(), keypair).await?.into_sender();
		Ok(Self { client, tx })
	}

	pub async fn with_keyfile(url: &str, keyfile: &Path) -> Result<Self> {
		let content =
			std::fs::read_to_string(keyfile).context("failed to read substrate keyfile")?;
		let secret = SecretUri::from_str(&content).context("failed to parse substrate keyfile")?;
		let keypair =
			Keypair::from_uri(&secret).context("substrate keyfile contains invalid suri")?;
		Self::new(url, keypair).await
	}
}

#[async_trait]
impl Runtime for SubxtClient {
	async fn get_block_time_in_ms(&self) -> Result<u64> {
		let runtime_call = timechain_runtime::apis().block_time_api().get_block_time_in_msec();
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: u64 = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		stream::empty().boxed()
	}

	async fn get_network(&self, network: NetworkId) -> Result<Option<(String, String)>> {
		let runtime_call = timechain_runtime::apis().networks_api().get_network(network);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: Option<(String, String)> = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_member_peer_id(
		&self,
		_: BlockHash,
		account: &AccountId,
	) -> Result<Option<PeerId>> {
		let account: subxt::utils::AccountId32 = subxt::utils::AccountId32(*(account.as_ref()));
		let runtime_call = timechain_runtime::apis().members_api().get_member_peer_id(account);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: Option<PeerId> = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_heartbeat_timeout(&self) -> Result<u64> {
		let runtime_call = timechain_runtime::apis().members_api().get_heartbeat_timeout();
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: u64 = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_min_stake(&self) -> Result<u128> {
		let runtime_call = timechain_runtime::apis().members_api().get_min_stake();
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: u128 = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_shards(&self, _: BlockHash, account: &AccountId) -> Result<Vec<ShardId>> {
		let account: subxt::utils::AccountId32 = subxt::utils::AccountId32(*(account.as_ref()));
		let runtime_call = timechain_runtime::apis().shards_api().get_shards(account);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: Vec<ShardId> = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_shard_members(
		&self,
		_: BlockHash,
		shard_id: ShardId,
	) -> Result<Vec<(AccountId, MemberStatus)>> {
		let runtime_call = timechain_runtime::apis().shards_api().get_shard_members(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: Vec<(AccountId, MemberStatus)> = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_shard_threshold(&self, _: BlockHash, shard_id: ShardId) -> Result<u16> {
		let runtime_call = timechain_runtime::apis().shards_api().get_shard_threshold(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: u16 = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_shard_status(
		&self,
		_: BlockHash,
		shard_id: ShardId,
	) -> Result<ShardStatus<BlockNumber>> {
		let runtime_call = timechain_runtime::apis().shards_api().get_shard_status(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: ShardStatus<BlockNumber> = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_shard_commitment(&self, _: BlockHash, shard_id: ShardId) -> Result<Commitment> {
		let runtime_call = timechain_runtime::apis().shards_api().get_shard_commitment(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: Commitment = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_shard_tasks(&self, _: BlockHash, shard_id: ShardId) -> Result<Vec<TaskExecution>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_shard_tasks(shard_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: Vec<TaskExecution> = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_task(&self, _: BlockHash, task_id: TaskId) -> Result<Option<TaskDescriptor>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_task(task_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: Option<TaskDescriptor> = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_task_signature(&self, task_id: TaskId) -> Result<Option<TssSignature>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_task_signature(task_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: Option<TssSignature> = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	async fn get_gateway(&self, network: NetworkId) -> Result<Option<Vec<u8>>> {
		let runtime_call = timechain_runtime::apis().tasks_api().get_gateway(network);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		let value: Option<Vec<u8>> = unsafe { std::mem::transmute(data) };
		Ok(value)
	}

	fn submit_register_member(
		&self,
		network: NetworkId,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Result<()> {
		self.tx.unbounded_send(Tx::RegisterMember { network, peer_id, stake_amount })?;
		Ok(())
	}

	fn submit_heartbeat(&self) -> Result<()> {
		self.tx.unbounded_send(Tx::Heartbeat)?;
		Ok(())
	}

	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: [u8; 65],
	) -> Result<()> {
		self.tx.unbounded_send(Tx::Commitment {
			shard_id,
			commitment,
			proof_of_knowledge,
		})?;
		Ok(())
	}

	fn submit_online(&self, shard_id: ShardId) -> Result<()> {
		self.tx.unbounded_send(Tx::Ready { shard_id })?;
		Ok(())
	}

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Result<()> {
		self.tx.unbounded_send(Tx::TaskSignature { task_id, signature })?;
		Ok(())
	}

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> Result<()> {
		self.tx.unbounded_send(Tx::TaskHash { task_id, cycle, hash })?;
		Ok(())
	}

	fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		result: TaskResult,
	) -> Result<()> {
		self.tx.unbounded_send(Tx::TaskResult { task_id, cycle, result })?;
		Ok(())
	}

	fn submit_task_error(&self, task_id: TaskId, cycle: TaskCycle, error: TaskError) -> Result<()> {
		self.tx.unbounded_send(Tx::TaskError { task_id, cycle, error })?;
		Ok(())
	}
}
