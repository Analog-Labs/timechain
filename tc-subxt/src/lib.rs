use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use subxt::backend::rpc::RpcClient;
use subxt::blocks::ExtrinsicEvents;
use subxt::client::OfflineClientT;
use subxt::dynamic::Value;
use subxt::ext::scale_value::Primitive;
use subxt::tx::SubmittableExtrinsic;
use subxt::tx::TxPayload;
use subxt::utils::{MultiAddress, MultiSignature, H256};
use subxt_signer::SecretUri;
use time_primitives::{
	AccountId, AccountInterface, ApiResult, BlockHash, BlockNumber, Commitment, MemberStatus,
	NetworkId, PeerId, ProofOfKnowledge, PublicKey, Runtime, ShardId, ShardStatus, SubmitResult,
	TaskCycle, TaskDescriptor, TaskError, TaskExecution, TaskId, TaskResult, TssPublicKey,
	TssSignature, TxBuilder,
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

#[derive(Clone)]
pub struct SubxtClient {
	// client connection to chain
	pub client: Arc<OnlineClient<PolkadotConfig>>,
	// rpc interface
	rpc: RpcClient,
	// signer use to sign transaction, Default is Alice
	signer: Arc<Keypair>,
	//maintains nocne of signer
	nonce: Arc<AtomicU64>,
}

impl SubxtClient {
	pub async fn new(url: &str, keyfile: Option<&Path>) -> Result<Self> {
		let rpc_client = RpcClient::from_url(url).await?;
		let api = OnlineClient::<PolkadotConfig>::from_rpc_client(rpc_client.clone()).await?;
		let content = if let Some(key) = keyfile {
			fs::read_to_string(key).context("failed to read substrate keyfile")?
		} else {
			"//Alice".into()
		};
		let secret = SecretUri::from_str(&content).context("failed to parse substrate keyfile")?;
		let keypair =
			Keypair::from_uri(&secret).context("substrate keyfile contains invalid suri")?;
		let account_id: subxt::utils::AccountId32 = keypair.public_key().into();
		let nonce = api.tx().account_nonce(&account_id).await?;
		Ok(Self {
			client: Arc::new(api),
			rpc: rpc_client,
			signer: Arc::new(keypair),
			nonce: Arc::new(AtomicU64::new(nonce)),
		})
	}

	pub async fn new_with_keypair(url: &str, keypair: Keypair) -> Result<Self> {
		let rpc_client = RpcClient::from_url(url).await?;
		let api = OnlineClient::<PolkadotConfig>::from_rpc_client(rpc_client.clone()).await?;
		let account_id: subxt::utils::AccountId32 = keypair.public_key().into();
		let nonce = api.tx().account_nonce(&account_id).await?;
		Ok(Self {
			client: Arc::new(api),
			rpc: rpc_client,
			signer: Arc::new(keypair),
			nonce: Arc::new(AtomicU64::new(nonce)),
		})
	}

	pub fn create_transfer_payload(
		dest: MultiAddress<AccountId32, ()>,
		value: u128,
	) -> subxt::tx::Payload<timechain_runtime::balances::calls::types::TransferKeepAlive> {
		timechain_runtime::tx().balances().transfer_keep_alive(dest, value)
	}

	pub fn create_withdraw_payload(
		from: AccountId32,
		value: u128,
		sequence: u64,
	) -> subxt::tx::Payload<timechain_runtime::timegraph::calls::types::Withdraw> {
		timechain_runtime::tx().timegraph().withdraw(from, value, sequence)
	}

	pub fn create_signed_payload<Call>(&self, call: &Call) -> Vec<u8>
	where
		Call: TxPayload,
	{
		self.client
			.tx()
			.create_signed_with_nonce(call, self.signer.as_ref(), self.nonce(), Default::default())
			.unwrap()
			.into_encoded()
	}

	pub async fn create_unsigned_payload<Call>(
		&self,
		call: &Call,
		address: &AccountId32,
	) -> Result<PartialExtrinsic<PolkadotConfig, OnlineClient<PolkadotConfig>>>
	where
		Call: TxPayload,
	{
		Ok(self
			.client
			.tx()
			.create_partial_signed(call, address, Default::default())
			.await?)
	}

	pub async fn add_signature_to_unsigned(
		&self,
		extrinsic: PartialExtrinsic<PolkadotConfig, OnlineClient<PolkadotConfig>>,
		address: &AccountId32,
		signature: [u8; 64],
	) -> Vec<u8> {
		let multi_address: MultiAddress<AccountId32, ()> = address.clone().into();
		let multi_signature = MultiSignature::Sr25519(signature);
		extrinsic
			.sign_with_address_and_signature(&multi_address, &multi_signature)
			.into_encoded()
	}

	pub async fn submit_transaction(&self, transaction: Vec<u8>) -> Result<H256> {
		let hash = SubmittableExtrinsic::from_bytes((*self.client).clone(), transaction)
			.submit()
			.await?;
		Ok(hash)
	}

	pub async fn sign_and_submit_watch<Call>(
		&self,
		call: &Call,
	) -> Result<ExtrinsicEvents<PolkadotConfig>>
	where
		Call: TxPayload,
	{
		Ok(self
			.client
			.tx()
			.sign_and_submit_then_watch_default(call, self.signer.as_ref())
			.await?
			.wait_for_finalized_success()
			.await?)
	}

	pub async fn sudo_sign_and_submit_watch(
		&self,
		call: RuntimeCall,
	) -> Result<ExtrinsicEvents<PolkadotConfig>> {
		let sudo_call = timechain_runtime::tx().sudo().sudo(call);
		Ok(self
			.client
			.tx()
			.sign_and_submit_then_watch_default(&sudo_call, self.signer.as_ref())
			.await?
			.wait_for_finalized_success()
			.await?)
	}

	pub async fn get_account_nonce(&self, id: [u8; 32]) {
		self.client.tx().account_nonce(&id.into()).await.unwrap();
	}

	pub async fn rpc(&self, method: &str, params: RpcParams) -> Result<()> {
		Ok(self.rpc.request(method, params).await?)
	}
}

impl AccountInterface for SubxtClient {
	fn nonce(&self) -> u64 {
		self.nonce.load(Ordering::SeqCst)
	}

	fn increment_nonce(&self) {
		self.nonce.fetch_add(1, Ordering::SeqCst);
	}

	fn public_key(&self) -> PublicKey {
		let public_key = self.signer.public_key();
		PublicKey::Sr25519(unsafe { std::mem::transmute(public_key) })
	}

	fn account_id(&self) -> AccountId {
		let account_id: subxt::utils::AccountId32 = self.signer.public_key().into();
		unsafe { std::mem::transmute(account_id) }
	}
}

impl TxBuilder for SubxtClient {
	fn submit_register_member(
		&self,
		network: NetworkId,
		public_key: PublicKey,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Vec<u8> {
		let network = unsafe { std::mem::transmute(network) };
		let public_key: MetadataMultiSigner = unsafe { std::mem::transmute(public_key) };
		let tx = timechain_runtime::tx().members().register_member(
			network,
			public_key,
			peer_id,
			stake_amount,
		);
		self.create_signed_payload(&tx)
	}

	fn submit_heartbeat(&self) -> Vec<u8> {
		let tx = timechain_runtime::tx().members().send_heartbeat();
		self.create_signed_payload(&tx)
	}

	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Vec<TssPublicKey>,
		proof_of_knowledge: [u8; 65],
	) -> Vec<u8> {
		let tx = timechain_runtime::tx()
			.shards()
			.commit(shard_id, commitment, proof_of_knowledge);
		self.create_signed_payload(&tx)
	}

	fn submit_online(&self, shard_id: ShardId) -> Vec<u8> {
		let tx = timechain_runtime::tx().shards().ready(shard_id);
		self.create_signed_payload(&tx)
	}

	fn submit_task_error(&self, task_id: TaskId, cycle: TaskCycle, error: TaskError) -> Vec<u8> {
		let error: task::TaskError = unsafe { std::mem::transmute(error) };
		let tx = timechain_runtime::tx().tasks().submit_error(task_id, cycle, error);
		self.create_signed_payload(&tx)
	}

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Vec<u8> {
		let tx = timechain_runtime::tx().tasks().submit_signature(task_id, signature);
		self.create_signed_payload(&tx)
	}

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> Vec<u8> {
		let tx = timechain_runtime::tx().tasks().submit_hash(task_id, cycle, hash);
		self.create_signed_payload(&tx)
	}

	fn submit_task_result(&self, task_id: TaskId, cycle: TaskCycle, status: TaskResult) -> Vec<u8> {
		let status: task::TaskResult = unsafe { std::mem::transmute(status) };
		let tx = timechain_runtime::tx().tasks().submit_result(task_id, cycle, status);
		self.create_signed_payload(&tx)
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
		todo!()
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

	async fn submit_register_member(
		&self,
		network: NetworkId,
		peer_id: PeerId,
		stake_amount: u128,
	) -> SubmitResult {
		let payload = <SubxtClient as TxBuilder>::submit_register_member(
			&self,
			network,
			self.public_key(),
			peer_id,
			stake_amount,
		);
		let _ = self.submit_transaction(payload).await;
		Ok(Ok(()))
	}

	async fn submit_heartbeat(&self) -> SubmitResult {
		let payload = <SubxtClient as TxBuilder>::submit_heartbeat(&self);
		let _ = self.submit_transaction(payload).await;
		Ok(Ok(()))
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

	async fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> SubmitResult {
		let payload = <SubxtClient as TxBuilder>::submit_commitment(
			&self,
			shard_id,
			commitment,
			proof_of_knowledge,
		);
		let _ = self.submit_transaction(payload).await;
		Ok(Ok(()))
	}

	async fn submit_online(&self, shard_id: ShardId) -> SubmitResult {
		let payload = <SubxtClient as TxBuilder>::submit_online(&self, shard_id);
		let _ = self.submit_transaction(payload).await;
		Ok(Ok(()))
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

	async fn submit_task_hash(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		hash: Vec<u8>,
	) -> SubmitResult {
		let payload = <SubxtClient as TxBuilder>::submit_task_hash(&self, task_id, cycle, hash);
		let _ = self.submit_transaction(payload).await;
		Ok(Ok(()))
	}

	async fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		status: TaskResult,
	) -> SubmitResult {
		let payload = <SubxtClient as TxBuilder>::submit_task_result(&self, task_id, cycle, status);
		let _ = self.submit_transaction(payload).await;
		Ok(Ok(()))
	}

	async fn submit_task_error(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> SubmitResult {
		let payload = <SubxtClient as TxBuilder>::submit_task_error(&self, task_id, cycle, error);
		let _ = self.submit_transaction(payload).await;
		Ok(Ok(()))
	}

	async fn submit_task_signature(
		&self,
		task_id: TaskId,
		signature: TssSignature,
	) -> SubmitResult {
		let payload = <SubxtClient as TxBuilder>::submit_task_signature(&self, task_id, signature);
		let _ = self.submit_transaction(payload).await;
		Ok(Ok(()))
	}
}
