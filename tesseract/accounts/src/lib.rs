use sp_core::{crypto::KeyTypeId, sr25519, sr25519::Public, Pair};
use sp_keystore::KeystorePtr;
use std::{
	error::Error,
	io::{Read, Write},
	str,
};

#[derive(Clone)]
pub struct Account {
	pub accounts: Public,
}

impl Account {
	//create a new account
	//the account is created with a random keypair
	pub fn new(password: &str, key_type: KeyTypeId, keystore: KeystorePtr) -> Self {
		let key_pair = sr25519::Pair::generate_with_phrase(Some(password));
		let pubkey = match keystore.sr25519_generate_new(key_type, Some(&key_pair.1)) {
			Ok(keypair) => keypair,
			Err(e) => {
				log::error!("Error generating keypair: {e:?}");
				panic!("Error generating keypair: {e:?}");
			},
		};

		Self { accounts: pubkey }
	}

	//get current account
	pub fn get_current_account(&self) -> Public {
		self.accounts
	}

	//store account to a json file
	pub fn gen_key_file(&self, path: &str) -> Result<(), Box<dyn Error>> {
		let mut file = std::fs::File::create(path)?;
		file.write_all(&self.accounts)?;
		Ok(())
	}

	//load account from a json file
	pub fn read_account_from_file(path: &str) -> Result<Public, Box<dyn Error>> {
		let mut file = std::fs::File::open(path).unwrap();
		let mut buffer = [0; 32];
		file.read_exact(&mut buffer).unwrap();
		let key = Public::from_raw(buffer);
		Ok(key)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sc_keystore::LocalKeystore;
	use std::{convert::TryFrom, sync::Arc};

	//tests account creation, keypair generation and keypair storage
	#[test]
	fn test_load_pubkey_file() {
		let key_type = KeyTypeId::try_from("ANLG").unwrap();
		let path = std::env::temp_dir().join("test_load_pubkey_file");
		let keystore: KeystorePtr = Arc::new(LocalKeystore::open(path, None).unwrap());
		let acc = Account::new("analog", key_type, keystore);
		let path = "../artifacts/test-account.json";
		acc.gen_key_file(path).unwrap();
		let account_str = Account::read_account_from_file(path).unwrap();
		assert_eq!(acc.accounts, account_str);
	}
}
