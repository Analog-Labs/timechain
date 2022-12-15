use keystore::commands::KeyTypeId;
use sp_core::{sr25519, sr25519::Public, Pair};
use sp_keystore::SyncCryptoStore;
use std::error::Error;
use std::io::{Read, Write};
use std::str;
use std::sync::Arc;

#[derive(Clone)]
pub struct Account {
    pub accounts: Public,
}

impl Account {
    //create a new account
    //the account is created with a random keypair
    pub fn new(password: &str, key_type: KeyTypeId, keystore: Arc<dyn SyncCryptoStore>) -> Self {
        let key_pair = sr25519::Pair::generate_with_phrase(Some(password));
        let pubkey =
            match SyncCryptoStore::sr25519_generate_new(&*keystore, key_type, Some(&key_pair.1)) {
                Ok(keypair) => keypair,
                Err(e) => {
                    log::error!("Error generating keypair: {:?}", e);
                    panic!("Error generating keypair: {:?}", e);
                }
            };

        Self { accounts: pubkey }
    }

    //get current account
    pub fn get_current_account(&self) -> Public {
        return self.accounts.clone();
    }

    //store account to a json file
    pub fn gen_key_file(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(&self.accounts.to_vec())?;
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
    use keystore::commands::KeyTypeId;
    use keystore::params::keystore_params::KeystoreParams;
    use sc_keystore::LocalKeystore;
    use sc_service::config::KeystoreConfig;
    use sp_keystore::SyncCryptoStorePtr;
    use std::{convert::TryFrom, env, sync::Arc};

    //tests account creation, keypair generation and keypair storage
    #[test]
    fn test_load_pubkey_file() {
        let keystore_params = KeystoreParams::default();

        let config_dir = env::current_dir().unwrap();
        let keystore = match keystore_params.keystore_config(&config_dir).unwrap() {
            (_, KeystoreConfig::Path { path, password }) => {
                // let public = with_crypto_scheme!(self.scheme, to_vec(&suri, password.clone()))?;
                let keystore: SyncCryptoStorePtr =
                    Arc::new(LocalKeystore::open(path, password).unwrap());
                keystore
            }
            _ => unreachable!("keystore_config always returns path and password; qed"),
        };
        let key_type = KeyTypeId::try_from("ANLG").unwrap();
        let acc = Account::new("analog", key_type, keystore);
        let path = "../artifacts/test-account.json";
        acc.gen_key_file(path).unwrap();
        let account_str = Account::read_account_from_file(path).unwrap();
        assert_eq!(acc.accounts, account_str);
    }
}
