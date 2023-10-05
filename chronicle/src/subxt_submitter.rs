use std::fs;
use subxt::ext::sp_core::{sr25519, Pair};
use subxt::tx::{PairSigner, TxPayload};
use subxt::{OnlineClient, PolkadotConfig};
use time_primitives::{
	Commitment, Network, PeerId, ProofOfKnowledge, PublicKey, ShardId, SubmitMembers, SubmitResult,
	SubmitShards, SubmitTasks, TaskCycle, TaskError, TaskId, TaskResult,
};
use std::sync::Arc;
use timechain_runtime::runtime_types::sp_runtime::MultiSigner as MetadataMultiSigner;
use timechain_runtime::runtime_types::time_primitives::shard;
use timechain_runtime::runtime_types::time_primitives::task;

#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/metadata.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain_runtime {}

pub type KeyPair = sp_core::sr25519::Pair;

pub struct TransactionSubmit {
	client: Arc<OnlineClient<PolkadotConfig>>,
	signer: Arc<PairSigner<PolkadotConfig, sr25519::Pair>>,
}

impl TransactionSubmit {
	pub async fn new(keyfile: String, password: Option<&str>) -> Self {
		tracing::info!("keyfile provided {:?}", keyfile);
		let content = fs::read_to_string(keyfile).unwrap();
		let seed_account: sr25519::Pair = sr25519::Pair::from_string(&content, password).unwrap();
		let seed_account_signer = PairSigner::<PolkadotConfig, sr25519::Pair>::new(seed_account);
		let api = OnlineClient::<PolkadotConfig>::from_url("ws://127.0.0.1:9944").await.unwrap();
		Self {
			client: Arc::new(api),
			signer: Arc::new(seed_account_signer),
		}
	}

	fn submit_tx<Call>(&self, call: &Call)
	where
		Call: TxPayload,
	{
		let data = futures::executor::block_on(self.client.tx().sign_and_submit_default(call, self.signer.as_ref()));
		tracing::info!("submitting transaction for call returned {:?}", data);
	}
}

impl Clone for TransactionSubmit {
	fn clone(&self) -> Self {
		Self {
			client: Arc::clone(&self.client),
			signer: Arc::clone(&self.signer),
		}
	}
}

impl SubmitMembers for TransactionSubmit {
	fn submit_register_member(
		&self,
		network: Network,
		public_key: PublicKey,
		peer_id: PeerId,
	) -> SubmitResult {
		let network: shard::Network = unsafe { std::mem::transmute(network) };
		let public_key: MetadataMultiSigner = unsafe { std::mem::transmute(public_key) };
		let tx = timechain_runtime::tx().members().register_member(network, public_key, peer_id);
		self.submit_tx(&tx);
		Ok(Ok(()))
	}

	fn submit_heartbeat(&self, _: PublicKey) -> SubmitResult {
		let tx = timechain_runtime::tx().members().send_heartbeat();
		self.submit_tx(&tx);
		Ok(Ok(()))
	}
}

impl SubmitShards for TransactionSubmit {
	fn submit_commitment(
		&self,
		shard_id: ShardId,
		_: PublicKey,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> SubmitResult {
		let tx = timechain_runtime::tx()
			.shards()
			.commit(shard_id, commitment, proof_of_knowledge);
		self.submit_tx(&tx);
		Ok(Ok(()))
	}

	fn submit_online(&self, shard_id: ShardId, _: PublicKey) -> SubmitResult {
		let tx = timechain_runtime::tx().shards().ready(shard_id);
		self.submit_tx(&tx);
		Ok(Ok(()))
	}
}

impl SubmitTasks for TransactionSubmit {
	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> SubmitResult {
		let tx = timechain_runtime::tx().tasks().submit_hash(task_id, cycle, hash);
		self.submit_tx(&tx);
		Ok(Ok(()))
	}

	fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		status: TaskResult,
	) -> SubmitResult {
		let status: task::TaskResult = unsafe { std::mem::transmute(status) };
		let tx = timechain_runtime::tx().tasks().submit_result(task_id, cycle, status);
		tracing::info!("{:?}", tx);
		self.submit_tx(&tx);
		Ok(Ok(()))
	}

	fn submit_task_error(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> SubmitResult {
		let error: task::TaskError = unsafe { std::mem::transmute(error) };
		let tx = timechain_runtime::tx().tasks().submit_error(task_id, cycle, error);
		self.submit_tx(&tx);
		Ok(Ok(()))
	}
}
