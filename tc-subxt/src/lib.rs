use std::fs;
use std::str::FromStr;
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

#[derive(Clone)]
pub struct SubxtClient {
	pub client: Arc<OnlineClient<PolkadotConfig>>,
	signer: Arc<Keypair>,
	nonce: u64,
}

impl SubxtClient {
	pub fn make_transaction<Call>(&self, call: &Call) -> Vec<u8>
	where
		Call: TxPayload,
	{
		self.client
			.tx()
			.create_signed_with_nonce(call, self.signer.as_ref(), self.nonce, Default::default())
			.unwrap()
			.into_encoded()
	}

	pub async fn new(keyfile: String) -> Self {
		let content = fs::read_to_string(keyfile).unwrap();
		let secret = SecretUri::from_str(&content).unwrap();
		let keypair = Keypair::from_uri(&secret).unwrap();

		let api = OnlineClient::<PolkadotConfig>::from_url("ws://127.0.0.1:9944").await.unwrap();
		Self {
			client: Arc::new(api),
			signer: Arc::new(keypair),
			nonce: 0,
		}
	}

	pub async fn submit_transaction(&self, transaction: &[u8]) {
		SubmittableExtrinsic::from_bytes((*self.client).clone(), transaction.to_vec())
			.submit()
			.await
			.unwrap();
	}
}
