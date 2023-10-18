use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use subxt::tx::TxPayload;
use subxt::{tx::SubmittableExtrinsic, OnlineClient, PolkadotConfig};
use subxt_signer::{sr25519::Keypair, SecretUri};
#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/metadata.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain_runtime {}
mod members;
mod shards;
mod tasks;
pub type KeyPair = sp_core::sr25519::Pair;

#[derive(Debug)]
pub enum TcSubxtError {
	InvalidFilePath,
	InvalidMnemonic,
	ClientIsntActive,
	InvalidAccountNonce,
}

#[derive(Clone)]
pub struct SubxtClient {
	client: Arc<OnlineClient<PolkadotConfig>>,
	signer: Arc<Keypair>,
	nonce: Arc<AtomicU64>,
}

impl SubxtClient {
	pub fn make_transaction<Call>(&mut self, call: &Call) -> Vec<u8>
	where
		Call: TxPayload,
	{
		let nonce = self.get_nonce();
		self.client
			.tx()
			.create_signed_with_nonce(call, self.signer.as_ref(), nonce, Default::default())
			.unwrap()
			.into_encoded()
	}

	pub async fn new(keyfile: PathBuf) -> Result<Self, TcSubxtError> {
		let content = fs::read_to_string(keyfile).map_err(|_| TcSubxtError::InvalidFilePath)?;
		let secret = SecretUri::from_str(&content).map_err(|_| TcSubxtError::InvalidMnemonic)?;
		let keypair = Keypair::from_uri(&secret).map_err(|_| TcSubxtError::InvalidMnemonic)?;
		let account_id: subxt::utils::AccountId32 = keypair.public_key().into();
		let api = OnlineClient::<PolkadotConfig>::from_url("ws://127.0.0.1:9944")
			.await
			.map_err(|_| TcSubxtError::ClientIsntActive)?;
		let nonce = api
			.tx()
			.account_nonce(&account_id)
			.await
			.map_err(|_| TcSubxtError::InvalidAccountNonce)?;
		Ok(Self {
			client: Arc::new(api),
			signer: Arc::new(keypair),
			nonce: Arc::new(AtomicU64::new(nonce)),
		})
	}

	pub async fn submit_transaction(&self, transaction: &[u8]) {
		SubmittableExtrinsic::from_bytes((*self.client).clone(), transaction.to_vec())
			.submit()
			.await
			.unwrap();
	}

	pub fn increment_nonce(&self) {
		self.nonce.fetch_add(1, Ordering::SeqCst);
	}

	pub fn get_nonce(&self) -> u64 {
		self.nonce.load(Ordering::SeqCst)
	}
}
