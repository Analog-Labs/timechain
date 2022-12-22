use crate::submit_to_timechain::TimechainSubmitter;
use accounts::Account;
use keystore::commands::KeyTypeId;
use sc_cli::Error;
use serde_json::Value;
use sp_core::{sr25519::Signature, Pair, Public};
use sp_keystore::SyncCryptoStore;
use std::{collections::HashMap, convert::TryFrom, sync::Arc};

pub async fn sign_data(
	acc: Account,
	msg: String,
	key_type: KeyTypeId,
	keystore: Arc<dyn SyncCryptoStore>,
	config: Arc<TimechainSubmitter>,
) -> Result<Signature, Box<dyn std::error::Error + Send + Sync>> {
	let sig_data = match SyncCryptoStore::sign_with(
		&*keystore,
		key_type,
		&acc.accounts.to_public_crypto_pair(),
		&msg.as_bytes(),
	) {
		Ok(sig) => match sig {
			Some(sig) => sig,
			None => return Err(Box::new(Error::from("Key doesn't exist"))),
		},
		Err(e) => {
			log::error!("Error signing data: {:?}", e);
			return Err(Box::new(e))
		},
	};

	//create signature
	let signature = match <sp_core::sr25519::Pair as Pair>::Signature::try_from(sig_data.as_slice())
		.map_err(|_| Error::SignatureFormatInvalid)
	{
		Ok(sig) => sig,
		Err(e) => {
			log::error!("Error creating signature: {:?}", e);
			return Err(Box::new(e))
		},
	};

	// deserialize message(event_data) to Log
	let sig_value = match serde_json::to_value(&signature) {
		Ok(v) => v,
		Err(e) => {
			log::error!("Error creating signature value: {:?}", e);
			return Err(Box::new(e))
		},
	};

	let pubkey_value = match serde_json::to_value(acc.accounts) {
		Ok(v) => v,
		Err(e) => {
			log::error!("Error creating pubkey value: {:?}", e);
			return Err(Box::new(e))
		},
	};

	let mut data = match serde_json::from_str::<HashMap<String, Value>>(&msg) {
		Ok(data) => data,
		Err(e) => return Err(e.into()),
	};

	data.insert("signature".to_string(), sig_value);
	data.insert("signer".to_string(), pubkey_value);

	let sig_bytes = serde_json::to_vec(&signature).unwrap();
	let data_bytes = serde_json::to_vec(&data).unwrap();
	match config.submit_data(sig_bytes.into(), data_bytes.into()).await {
		Ok(s) => s,
		Err(e) => {
			log::error!("Error submitting to timechain: {:?}", e);
			return Err(e.into())
		},
	};

	Ok(signature)
}

pub async fn verify_data(
	sig: Signature,
	msg: String,
	pubkey: sp_core::sr25519::Public,
) -> Result<(), Box<dyn std::error::Error>> {
	//check if the signature message and public key are valid
	if <sp_core::sr25519::Pair as Pair>::verify(&sig, &msg, &pubkey) {
		log::info!("Signature verifies correctly.");
		Ok(())
	} else {
		log::error!("Signature invalid.");
		return Err("Signature invalid/incorrect".into())
	}
}

// #[cfg(test)]
// mod tests {
//     use crate::submit_to_timechain::timechain::runtime_types::pallet_tesseract_sig_storage::types::TesseractRole;

//     use super::*;
//     use keystore::commands::KeyTypeId;
//     use keystore::params::keystore_params::KeystoreParams;
//     use sc_keystore::LocalKeystore;
//     use sc_service::config::KeystoreConfig;
//     use sp_keystore::SyncCryptoStorePtr;
//     use std::env;

//     #[tokio::test]
//     async fn test_sign_event_data() {
//         let keystore_params = KeystoreParams::default();
//         let config_dir = env::current_dir().unwrap();
//         let keystore = match keystore_params.keystore_config(&config_dir).unwrap() {
//             (_, KeystoreConfig::Path { path, password }) => {
//                 let keystore: SyncCryptoStorePtr =
//                     Arc::new(LocalKeystore::open(path, password).unwrap());
//                 keystore
//             }
//             _ => unreachable!("keystore_config always returns path and password; qed"),
//         };
//         let key_type_str = "anlg";
//         let key_type = KeyTypeId::try_from(key_type_str).unwrap();
//         let acc = Account::new("analog", key_type, keystore.clone());
//         let submitter = TimechainSubmitter::default_config().await.unwrap();
//         let config = Arc::new(submitter);
//         let msg =
// r#"{"address":"0x0000000000000000000000000000000000000000","topics":["
// 0x0000000000000000000000000000000000000000000000000000000000000000"],"data":"
// 0x0000000000000000000000000000000000000000000000000000000000000000","block_hash":null,"
// block_number":null,"transaction_hash":null,"transaction_index":null,"log_index":null,"
// transaction_log_index":null,"log_type":null,"removed":null}"#;         match
// config.add_member(TesseractRole::Aggregator).await {             Ok(_) => {
//                 println!("Collector added successfully to timechain");
//             }
//             Err(e) => {
//                 println!("Error adding Collector to timechain: {:?}", e);
//             }
//         };
//         let sig = sign_data(
//             acc.clone(),
//             msg.to_string(),
//             key_type,
//             keystore.clone(),
//             config.clone(),
//         )
//         .await
//         .unwrap();
//         match verify_data(sig, msg.to_string(), acc.accounts).await {
//             Ok(_d) => assert!(true),
//             Err(_e) => assert!(false),
//         };
//     }
// }
