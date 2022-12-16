use crate::submit_to_timechain::timechain::runtime_types;
use crate::submit_to_timechain::timechain::runtime_types::pallet_tesseract_sig_storage::types::TesseractRole;
use crate::submit_to_timechain::timechain::sudo;
use sp_core::crypto::AccountId32;
use sp_keyring::sr25519::sr25519::Pair;
use sp_keyring::AccountKeyring;
use subxt::tx::PairSigner;
use subxt::{OnlineClient, PolkadotConfig};

type Call = runtime_types::timechain_runtime::RuntimeCall;
type TesseractSigStorageCall = runtime_types::pallet_tesseract_sig_storage::pallet::Call;

#[subxt::subxt(
    runtime_metadata_path = "../artifacts/timechain-metadata.scale",
    derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain {}

pub struct TimechainSubmitter {
    pub signer: PairSigner<PolkadotConfig, Pair>,
    pub api: OnlineClient<PolkadotConfig>,
    pub account: AccountId32,
}

impl TimechainSubmitter {
    pub async fn default_config() -> Result<Self, Box<dyn std::error::Error>> {
        tracing_subscriber::fmt::init();
        let signer = PairSigner::new(AccountKeyring::Alice.pair());
        let api = OnlineClient::<PolkadotConfig>::new().await.unwrap();
        let account = AccountKeyring::Alice.to_account_id();
        Ok(Self {
            signer,
            api,
            account,
        })
    }

    pub fn check_metadata(&self) -> bool {
        match timechain::validate_codegen(&self.api) {
            Ok(_) => true,
            Err(e) => {
                log::error!("Error validating codegen: {:?}", e);
                false
            }
        }
    }

    pub async fn add_member(
        &self,
        role: TesseractRole,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let account = AccountKeyring::Alice.to_account_id();
        let call = Call::TesseractSigStorage(TesseractSigStorageCall::add_member { account, role });
        let tx = timechain::tx().sudo().sudo(call);
        let tx_progress = match self
            .api
            .tx()
            .sign_and_submit_then_watch_default(&tx, &self.signer)
            .await
        {
            Ok(d) => d,
            Err(e) => {
                log::error!("Error submitting transaction: {:?}", e);
                return Err(Box::new(e));
            }
        };
        let events = match tx_progress.wait_for_finalized_success().await {
            Ok(ev) => ev,
            Err(e) => {
                log::error!("Error waiting for transaction to finalize: {:?}", e);
                return Err(Box::new(e));
            }
        };
        match events.has::<sudo::events::Sudid>() {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn fetch_members(
        &self,
    ) -> Result<TesseractRole, Box<dyn std::error::Error + Send + Sync>> {
        let members = timechain::storage()
            .tesseract_sig_storage()
            .tesseract_members_root();
        let mut iter = self.api.storage().iter(members, 10, None).await?;
        match iter.next().await {
            Ok(Some((key, value))) => {
                log::info!("{}: {:?}", hex::encode(&key), value);
                return Ok(value);
            }
            Ok(None) => {
                log::error!("No members found");
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No members found",
                )));
            }
            Err(e) => {
                log::error!("Error fetching members: {:?}", e);
                return Err(Box::new(e));
            }
        }
    }

    pub async fn remove_member(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let account = AccountKeyring::Alice.to_account_id();
        let call = Call::TesseractSigStorage(TesseractSigStorageCall::remove_member { account });
        let tx = timechain::tx().sudo().sudo(call);
        let tx_progress = match self
            .api
            .tx()
            .sign_and_submit_then_watch_default(&tx, &self.signer)
            .await
        {
            Ok(d) => d,
            Err(e) => {
                log::error!("Error submitting transaction: {:?}", e);
                return Err(Box::new(e));
            }
        };
        let events = match tx_progress.wait_for_finalized_success().await {
            Ok(ev) => ev,
            Err(e) => {
                log::error!("Error waiting for transaction to finalize: {:?}", e);
                return Err(Box::new(e));
            }
        };
        match events.has::<sudo::events::Sudid>() {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn submit_data(
        &self,
        sig: Vec<u8>,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let api = self.api.tx().to_owned();
        let network_id = "123".to_string(); // dummy field can't use it right now
        let block_height = 1; // initial block
        let tx = timechain::tx().tesseract_sig_storage().store_signature(
            data.clone(),
            network_id.as_bytes().to_vec(),
            block_height,
        );

        match api.sign_and_submit_default(&tx, &self.signer).await {
            Ok(d) => {
                log::info!("Data successfully submitted with hash: {:?}", d);
                return Ok(());
            }
            Err(e) => {
                log::error!("Error submitting transaction: {:?}", e);
                return Err(Box::new(e));
            }
        }
    }
}

// #[tokio::test]
// async fn test_timechain_submitter() {
//     let submitter = TimechainSubmitter::default_config().await.unwrap();
//     let role = TesseractRole::Collector;
//     let signature = "v60ac9b307991d53fb6d5cf1f0dfdc566e8727dfe2c64616821ddf1e8e3483a18224dbd87485c69f2e9826349996071f01e21adc8e920277063b645ff9c762f81";
//     let msg = r#"{"address":"0x0000000000000000000000000000000000000000","topics":["0x0000000000000000000000000000000000000000000000000000000000000000"],"data":"0x0000000000000000000000000000000000000000000000000000000000000000","block_hash":null,"block_number":null,"transaction_hash":null,"transaction_index":null,"log_index":null,"transaction_log_index":null,"log_type":null,"removed":null}"#;
//     let sig = signature.as_bytes();
//     let data = msg.as_bytes();
//     assert!(submitter.check_metadata());
//     match submitter.add_member(role).await {
//         Ok(_d) => assert!(true),
//         Err(e) => {
//             println!("{:?}", e);
//             assert!(false);
//         }
//     };
//     match submitter.submit_data(sig.into(), data.into()).await {
//         Ok(_d) => assert!(true),
//         Err(e) => {
//             println!("{:?}", e);
//             assert!(false);
//         }
//     };
//     match submitter.fetch_members().await {
//         Ok(_d) => assert!(true),
//         Err(e) => {
//             println!("{:?}", e);
//             assert!(false);
//         }
//     };
//     match submitter.remove_member().await {
//         Ok(_d) => assert!(true),
//         Err(e) => {
//             println!("{:?}", e);
//             assert!(false);
//         }
//     };
// }
