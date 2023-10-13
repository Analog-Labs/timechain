use std::fs;
use std::str::FromStr;
use std::sync::Arc;
use subxt::{
	backend::Backend, constants::ConstantsClient, tx::SubmittableExtrinsic, OnlineClient,
	PolkadotConfig,
};
use subxt_signer::{bip39::Mnemonic, sr25519::Keypair, SecretUri};
use time_primitives::{
	Commitment, Network, PeerId, ProofOfKnowledge, PublicKey, ShardId, SubmitMembers, SubmitResult,
	SubmitShards, SubmitTasks, TaskCycle, TaskError, TaskId, TaskResult,
};
use timechain_runtime::runtime_types::sp_runtime::MultiSigner as MetadataMultiSigner;
use timechain_runtime::runtime_types::time_primitives::shard;
use timechain_runtime::runtime_types::time_primitives::task;
#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/metadata.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain_runtime {}
pub type KeyPair = sp_core::sr25519::Pair;
#[derive(Clone)]
pub struct SubxtBuilder {
	client: Arc<OnlineClient<PolkadotConfig>>,
	signer: Option<Arc<Keypair>>,
}

impl SubxtBuilder {
	pub async fn make_transaction() {}

	pub async fn new(keyfile: Option<String>) -> Self {
		let keypair = if let Some(path) = keyfile {
			let content = fs::read_to_string(path).unwrap();
			let secret = SecretUri::from_str(&content).unwrap();
			let keypair = Keypair::from_uri(&secret).unwrap();
			Some(Arc::new(keypair))
		} else {
			None
		};

		let api = OnlineClient::<PolkadotConfig>::from_url("ws://127.0.0.1:9944").await.unwrap();
		Self {
			client: Arc::new(api),
			signer: keypair,
		}
	}

	pub async fn register_member(network: Network, public_key: PublicKey, peer_id: PeerId) {
		let network: shard::Network = unsafe { std::mem::transmute(network) };
		let public_key: MetadataMultiSigner = unsafe { std::mem::transmute(public_key) };
		let tx = timechain_runtime::tx().members().register_member(network, public_key, peer_id);
		self.submit_tx(&tx).await;
		Ok(Ok(()))
	}

	pub async fn submit_commitment(
		shard_id: ShardId,
		member: PublicKey,
		commitment: Vec<[u8; 33]>,
		proof_of_knowledge: [u8; 65],
	) {
		let tx = timechain_runtime::tx()
			.shards()
			.commit(shard_id, commitment, proof_of_knowledge);
	}

	pub async fn submit_heartbeat(public_key: PublicKey) {
		let tx = timechain_runtime::tx().members().send_heartbeat();
	}

	pub async fn submit_ready(shard_id: ShardId, member: PublicKey) {
		let tx = timechain_runtime::tx().shards().ready(shard_id);
	}

	pub async fn submit_tash_error(task_id: TaskId, cycle: TaskCycle, error: TaskError) {
		let error: task::TaskError = unsafe { std::mem::transmute(error) };
		let tx = timechain_runtime::tx().tasks().submit_error(task_id, cycle, error);
	}

	pub async fn submit_tash_hash(task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) {
		let tx = timechain_runtime::tx().tasks().submit_hash(task_id, cycle, hash);
	}

	pub async fn submit_task_result(task_id: TaskId, cycle: TaskCycle, status: TaskResult) {
		let status: task::TaskResult = unsafe { std::mem::transmute(status) };
		let tx = timechain_runtime::tx().tasks().submit_result(task_id, cycle, status);
	}

	pub async fn submit_transaction(&self, transaction: &[u8]) {
		SubmittableExtrinsic::from_bytes((*self.client).clone(), transaction.to_vec())
			.submit()
			.await
			.unwrap();
	}
}
