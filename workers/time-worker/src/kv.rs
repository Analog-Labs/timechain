use log::warn;
use sp_core::{keccak_256, Pair};
use sp_keystore::KeystorePtr;
use std::convert::{From, TryInto};
use time_primitives::{
	crypto::{Public as TimePublic, Signature},
	KEY_TYPE,
};

#[derive(Clone)]
pub struct TimeKeyvault(Option<KeystorePtr>);

impl TimeKeyvault {
	pub fn authority_id(&self, _keys: &[TimePublic]) -> Option<sp_core::sr25519::Public> {
		let store = self.0.clone()?;

		let public = store.sr25519_public_keys(KEY_TYPE);
		if public.len() > 1 {
			warn!(target: crate::TW_LOG, "Multiple private keys found!");
		}
		public.get(0).cloned()
	}

	pub fn sign(&self, public: &TimePublic, message: &[u8]) -> Option<Signature> {
		let store = self.0.clone()?;

		let msg = keccak_256(message);
		let sig = store.sr25519_sign(KEY_TYPE, &public.clone().into(), &msg).ok()??;

		// check that `sig` has the expected result type
		let sig = sig.try_into().ok()?;

		Some(sig)
	}

	pub fn verify(public: &TimePublic, sig: &Signature, message: &[u8]) -> bool {
		let msg = keccak_256(message);
		let sig = sig.as_ref();
		let public = public.as_ref();

		sp_core::sr25519::Pair::verify(sig, msg, public)
	}

	pub fn public_keys(&self) -> Vec<TimePublic> {
		let store = self.0.clone().unwrap();
		store
			.sr25519_public_keys(KEY_TYPE)
			.iter()
			.map(|k| TimePublic::from(*k))
			.collect()
	}

	pub fn authority_keys(&self) -> Vec<sp_core::ed25519::Public> {
		let store = self.0.clone().unwrap();
		store.ed25519_public_keys(sp_consensus_grandpa::KEY_TYPE).to_vec()
	}

	pub fn authority_sign(
		&self,
		public: &sp_core::ed25519::Public,
		message: &[u8],
	) -> Option<sp_core::ed25519::Signature> {
		let store = self.0.clone()?;

		let msg = keccak_256(message);
		let sig = store.ed25519_sign(sp_consensus_grandpa::KEY_TYPE, &public, &msg).ok()??;

		Some(sig)
	}

	pub fn get_store(&self) -> Option<KeystorePtr> {
		self.0.clone()
	}
}

impl From<Option<KeystorePtr>> for TimeKeyvault {
	fn from(store: Option<KeystorePtr>) -> TimeKeyvault {
		TimeKeyvault(store)
	}
}
