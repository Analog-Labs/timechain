use std::fs;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use subxt::tx::TxPayload;
use subxt::{
	backend::Backend, constants::ConstantsClient, tx::SubmittableExtrinsic, OnlineClient,
	PolkadotConfig,
};
use subxt_signer::{bip39::Mnemonic, sr25519::Keypair, SecretUri};
#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/metadata.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain_runtime {}
mod members;
mod shards;
mod tasks;
pub type KeyPair = sp_core::sr25519::Pair;

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
		let tx_bytes = self
			.client
			.tx()
			.create_signed_with_nonce(call, self.signer.as_ref(), nonce, Default::default())
			.unwrap()
			.into_encoded();
		self.increment_nonce();
		tx_bytes
	}

	pub async fn new(keyfile: String) -> Self {
		let content = fs::read_to_string(keyfile).expect("file path not found");
		let secret =
			SecretUri::from_str(&content).expect("cannot create secret from content of file");
		let keypair = Keypair::from_uri(&secret).expect("cannot create keypair from secret");
		let account_id: subxt::utils::AccountId32 = keypair.public_key().into();
		let api = OnlineClient::<PolkadotConfig>::from_url("ws://127.0.0.1:9944").await.unwrap();
		let nonce = api.tx().account_nonce(&account_id).await.unwrap();
		Self {
			client: Arc::new(api),
			signer: Arc::new(keypair),
			nonce: Arc::new(AtomicU64::new(0)),
		}
	}

	pub async fn submit_transaction(&self, transaction: &[u8]) {
		SubmittableExtrinsic::from_bytes((*self.client).clone(), transaction.to_vec())
			.submit()
			.await
			.unwrap();
	}
	// A method to increment the nonce, as an example
	pub fn increment_nonce(&self) {
		self.nonce.fetch_add(1, Ordering::SeqCst);
	}

	// A method to get the current value of the nonce
	pub fn get_nonce(&self) -> u64 {
		self.nonce.load(Ordering::SeqCst)
	}
}

impl Clone for SubxtClient {
	fn clone(&self) -> Self {
		Self {
			client: self.client.clone(),
			signer: self.signer.clone(),
			nonce: self.nonce.clone(),
		}
	}
}
