use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use subxt::backend::rpc::{RpcClient, RpcParams};
use subxt::tx::TxPayload;
use subxt::utils::H256;
use subxt::{tx::SubmittableExtrinsic, OnlineClient, PolkadotConfig};
use subxt_signer::{sr25519::Keypair, SecretUri};
use time_primitives::{AccountId, PublicKey};

#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/metadata.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub(crate) mod timechain_runtime {}

mod members;
mod shards;
mod tasks;

pub use subxt::backend::rpc::rpc_params;
pub use timechain_runtime::runtime_types::time_primitives::shard::{Network, ShardStatus};
pub use timechain_runtime::runtime_types::time_primitives::task::{
	Function, TaskDescriptor, TaskDescriptorParams, TaskStatus,
};
pub use timechain_runtime::tasks::events::TaskCreated;

pub trait AccountInterface {
	fn nonce(&self) -> u64;
	fn increment_nonce(&self);
	fn public_key(&self) -> PublicKey;
	fn account_id(&self) -> AccountId;
}

#[derive(Clone)]
pub struct SubxtClient {
	// client connection to chain
	client: Arc<OnlineClient<PolkadotConfig>>,
	// rpc interface
	rpc: RpcClient,
	// signer use to sign transaction, Default is Alice
	signer: Arc<Keypair>,
	//maintains nocne of signer
	nonce: Arc<AtomicU64>,
}

impl SubxtClient {
	pub fn make_transaction<Call>(&self, call: &Call) -> Vec<u8>
	where
		Call: TxPayload,
	{
		self.client
			.tx()
			.create_signed_with_nonce(call, self.signer.as_ref(), self.nonce(), Default::default())
			.unwrap()
			.into_encoded()
	}

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

	pub async fn submit_transaction(&self, transaction: Vec<u8>) -> Result<H256> {
		let hash = SubmittableExtrinsic::from_bytes((*self.client).clone(), transaction)
			.submit()
			.await?;
		Ok(hash)
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
