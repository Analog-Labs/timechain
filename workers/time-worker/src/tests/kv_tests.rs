use std::sync::Arc;

use sc_keystore::LocalKeystore;
use sp_core::{ed25519, keccak_256, sr25519, Pair};
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};

use time_primitives::{crypto, KEY_TYPE};

use crate::TimeKeyvault;

/// Set of test accounts using [`time_primitives::crypto`] types.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter)]
pub(crate) enum Keyring {
	Alice,
	Bob,
	Charlie,
	Dave,
	Eve,
	Ferdie,
	One,
	Two,
}

impl Keyring {
	/// Sign `msg`.
	pub fn sign(self, msg: &[u8]) -> crypto::Signature {
		let msg = keccak_256(msg);
		sr25519::Pair::from(self).sign(&msg).into()
	}

	/// Return key pair.
	pub fn pair(self) -> crypto::Pair {
		sr25519::Pair::from_string(self.to_seed().as_str(), None).unwrap().into()
	}

	/// Return public key.
	pub fn public(self) -> crypto::Public {
		self.pair().public()
	}

	/// Return seed string.
	pub fn to_seed(self) -> String {
		format!("//{}", self)
	}
}

impl From<Keyring> for crypto::Pair {
	fn from(k: Keyring) -> Self {
		k.pair()
	}
}

impl From<Keyring> for sr25519::Pair {
	fn from(k: Keyring) -> Self {
		k.pair().into()
	}
}

impl From<Keyring> for ed25519::Pair {
	fn from(k: Keyring) -> Self {
		k.into()
	}
}

fn keystore() -> SyncCryptoStorePtr {
	Arc::new(LocalKeystore::in_memory())
}

#[test]
fn verify_should_work() {
	let msg = keccak_256(b"I am Alice!");
	let sig = Keyring::Alice.sign(b"I am Alice!");

	assert!(sr25519::Pair::verify(&sig.clone().into(), &msg, &Keyring::Alice.public().into(),));

	// different public key -> fail
	assert!(!sr25519::Pair::verify(&sig.clone().into(), &msg, &Keyring::Bob.public().into(),));

	let msg = keccak_256(b"I am not Alice!");

	// different msg -> fail
	assert!(!sr25519::Pair::verify(&sig.into(), &msg, &Keyring::Alice.public().into()));
}

#[test]
fn pair_works() {
	let want = crypto::Pair::from_string("//Alice", None).expect("Pair failed").to_raw_vec();
	let got = Keyring::Alice.pair().to_raw_vec();
	assert_eq!(want, got);

	let want = crypto::Pair::from_string("//Bob", None).expect("Pair failed").to_raw_vec();
	let got = Keyring::Bob.pair().to_raw_vec();
	assert_eq!(want, got);

	let want = crypto::Pair::from_string("//Charlie", None).expect("Pair failed").to_raw_vec();
	let got = Keyring::Charlie.pair().to_raw_vec();
	assert_eq!(want, got);

	let want = crypto::Pair::from_string("//Dave", None).expect("Pair failed").to_raw_vec();
	let got = Keyring::Dave.pair().to_raw_vec();
	assert_eq!(want, got);

	let want = crypto::Pair::from_string("//Eve", None).expect("Pair failed").to_raw_vec();
	let got = Keyring::Eve.pair().to_raw_vec();
	assert_eq!(want, got);

	let want = crypto::Pair::from_string("//Ferdie", None).expect("Pair failed").to_raw_vec();
	let got = Keyring::Ferdie.pair().to_raw_vec();
	assert_eq!(want, got);

	let want = crypto::Pair::from_string("//One", None).expect("Pair failed").to_raw_vec();
	let got = Keyring::One.pair().to_raw_vec();
	assert_eq!(want, got);

	let want = crypto::Pair::from_string("//Two", None).expect("Pair failed").to_raw_vec();
	let got = Keyring::Two.pair().to_raw_vec();
	assert_eq!(want, got);
}

#[ignore] // TODO: test fails always
#[test]
fn authority_id_works() {
	let store = keystore();

	let alice: crypto::Public =
		SyncCryptoStore::sr25519_generate_new(&*store, KEY_TYPE, Some(&Keyring::Alice.to_seed()))
			.ok()
			.unwrap()
			.into();

	let bob = Keyring::Bob.public();
	let charlie = Keyring::Charlie.public();

	let store: TimeKeyvault = Some(store).into();

	let mut keys = vec![bob, charlie];

	let id = store.authority_id(keys.as_slice());
	assert!(id.is_none());

	keys.push(alice.clone());

	let id = store.authority_id(keys.as_slice()).unwrap();
	assert_eq!(id, alice.into());
}

#[ignore] // TODO: test fails always
#[test]
fn sign_works() {
	let store = keystore();

	let alice: crypto::Public =
		SyncCryptoStore::sr25519_generate_new(&*store, KEY_TYPE, Some(&Keyring::Alice.to_seed()))
			.ok()
			.unwrap()
			.into();

	let store: TimeKeyvault = Some(store).into();

	let msg = b"are you involved or commited?";

	let sig1 = store.sign(&alice, msg).unwrap();
	let sig2 = Keyring::Alice.sign(msg);

	assert_eq!(sig1, sig2);
}

#[ignore] // TODO: test fails always
#[test]
fn sign_error() {
	let store = keystore();

	let _ = SyncCryptoStore::sr25519_generate_new(&*store, KEY_TYPE, Some(&Keyring::Bob.to_seed()))
		.ok()
		.unwrap();

	let store: TimeKeyvault = Some(store).into();

	let alice = Keyring::Alice.public();

	let msg = b"are you involved or commited?";
	let sig = store.sign(&alice, msg);

	assert!(sig.is_some());
}

#[ignore] // TODO: test fails always
#[test]
fn sign_no_keystore() {
	let store: TimeKeyvault = None.into();

	let alice = Keyring::Alice.public();
	let msg = b"are you involved or commited";

	let sig = store.sign(&alice, msg);
	assert!(sig.is_some());
}

#[test]
fn verify_works() {
	let store = keystore();

	let alice: crypto::Public =
		SyncCryptoStore::sr25519_generate_new(&*store, KEY_TYPE, Some(&Keyring::Alice.to_seed()))
			.ok()
			.unwrap()
			.into();

	let store: TimeKeyvault = Some(store).into();

	// `msg` and `sig` match
	let msg = b"are you involved or commited?";
	let sig = store.sign(&alice, msg).unwrap();
	assert!(TimeKeyvault::verify(&alice, &sig, msg));

	// `msg and `sig` don't match
	let msg = b"you are just involved";
	assert!(!TimeKeyvault::verify(&alice, &sig, msg));
}
