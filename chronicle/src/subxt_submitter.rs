use std::fs;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::{ bip39::Mnemonic, sr25519::Keypair, SecretUri };
use time_primitives::{
	Commitment, Network, PeerId, ProofOfKnowledge, PublicKey, ShardId, SubmitMembers, SubmitResult,
	SubmitShards, SubmitTasks, TaskCycle, TaskError, TaskId, TaskResult,
};
use subxt::tx::TxPayload;
use std::sync::Arc;
use timechain_runtime::runtime_types::sp_runtime::MultiSigner as MetadataMultiSigner;
use timechain_runtime::runtime_types::time_primitives::shard;
use timechain_runtime::runtime_types::time_primitives::task;
use std::str::FromStr;

#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/metadata.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain_runtime {}

pub type KeyPair = sp_core::sr25519::Pair;

#[derive(Clone)]
pub struct TransactionSubmit {
	client: Arc<OnlineClient<PolkadotConfig>>,
	signer: Arc<Keypair>,
}


impl TransactionSubmit {
	pub async fn new(keyfile: String, password: Option<&str>) -> Self {
		let content = fs::read_to_string(keyfile).unwrap();
		let secret = SecretUri::from_str(&content).unwrap();
		let keypair = Keypair::from_uri(&secret).unwrap();
		let api = OnlineClient::<PolkadotConfig>::from_url("ws://127.0.0.1:9944").await.unwrap();
		Self {
			client: Arc::new(api),
			signer: Arc::new(keypair),
		}
	}

	async fn submit_tx<Call>(&self, call: &Call)
	where
		Call: TxPayload,
	{
		let data = self.client.tx().sign_and_submit_default(call, self.signer.as_ref()).await;
		tracing::info!("submitting transaction for call returned {:?}", data);
	}
}

#[async_trait::async_trait]
impl SubmitMembers for TransactionSubmit {
	async fn submit_register_member(
		&self,
		network: Network,
		public_key: PublicKey,
		peer_id: PeerId,
	) -> SubmitResult {
		let network: shard::Network = unsafe { std::mem::transmute(network) };
		let public_key: MetadataMultiSigner = unsafe { std::mem::transmute(public_key) };
		let tx = timechain_runtime::tx().members().register_member(network, public_key, peer_id);
		self.submit_tx(&tx).await;
		Ok(Ok(()))
	}

	async fn submit_heartbeat(&self, _: PublicKey) -> SubmitResult {
		let tx = timechain_runtime::tx().members().send_heartbeat();
		self.submit_tx(&tx).await;
		Ok(Ok(()))
	}
}

#[async_trait::async_trait]
impl SubmitShards for TransactionSubmit {
	async fn submit_commitment(
		&self,
		shard_id: ShardId,
		_: PublicKey,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> SubmitResult {
		let tx = timechain_runtime::tx()
			.shards()
			.commit(shard_id, commitment, proof_of_knowledge);
		self.submit_tx(&tx).await;
		Ok(Ok(()))
	}

	async fn submit_online(&self, shard_id: ShardId, _: PublicKey) -> SubmitResult {
		let tx = timechain_runtime::tx().shards().ready(shard_id);
		self.submit_tx(&tx).await;
		Ok(Ok(()))
	}
}

#[async_trait::async_trait]
impl SubmitTasks for TransactionSubmit {
	async fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> SubmitResult {
		let tx = timechain_runtime::tx().tasks().submit_hash(task_id, cycle, hash);
		self.submit_tx(&tx).await;
		Ok(Ok(()))
	}

	async fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		status: TaskResult,
	) -> SubmitResult {
		let status: task::TaskResult = unsafe { std::mem::transmute(status) };
		let tx = timechain_runtime::tx().tasks().submit_result(task_id, cycle, status);
		tracing::info!("{:?}", tx);
		self.submit_tx(&tx).await;
		Ok(Ok(()))
	}

	async fn submit_task_error(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> SubmitResult {
		let error: task::TaskError = unsafe { std::mem::transmute(error) };
		let tx = timechain_runtime::tx().tasks().submit_error(task_id, cycle, error);
		self.submit_tx(&tx).await;
		Ok(Ok(()))
	}
}
