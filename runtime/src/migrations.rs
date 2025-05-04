use core::marker::PhantomData;

use polkadot_sdk::*;

use frame_support::traits::{Get, OnRuntimeUpgrade};
use sp_core::hex2array;
use sp_runtime::AccountId32;

use crate::AccountId;

const MIGRATED: [[u8; 32]; 34] = [
	hex2array!("b861b3ed11422a929a13c87a2f8e55ae322166252c507ba6d5c1cf6f8b226073"),
	hex2array!("bc4ac8e007768f477ac96eda894aa8d1b09c0db24ae7ae9523c6f159c7fac76f"),
	hex2array!("62cc4557dcd57129cef21809f35781f13f7484d6c9433ef89d4e1df74b59296f"),
	hex2array!("5671f1865409730ac94b019ecb72c53754fb7b8623092ae5c64bb86a57b47b54"),
	hex2array!("760dbc8a48b51e9b7dca6d19f58db7f480fc03f95d67d73c7309bcb25c322c4b"),
	hex2array!("d66ee38c1233e2b4853c9e05bd683596966601b355e06854a817d44f74d5db6f"),
	hex2array!("8827c8710ca030e4db1b45a4fdcafd79e9168221b15ea603e46ef5deff87bb1b"),
	hex2array!("5046f39d167dcb62c45f81ffb28ee71c9515675f36c754d943d47c6308342926"),
	hex2array!("a6d68d69a06b0b3663460b86f76329f0dc0cb61b19e43d13639e25ce0880db6b"),
	hex2array!("eabf189105ff0b616d0b124025826b79b7133838412332d971d309b950fd8d5e"),
	hex2array!("c401e6db2a4884d438009a5030e310306350de3893776abc02ce39629fcf8c6b"),
	hex2array!("36947a8f1545d0d7f108ca32f01cd917e2b49b80f40c26c6824f132c40f9e609"),
	hex2array!("3097d8fc3bc253716333b4ef327956a4ed4b6544bd79e036788eb81c2ff01253"),
	hex2array!("baad8669c69e503d91743430fa8172c2c18e343da58efa5d1245972eb3da887b"),
	hex2array!("4a14466134feeefeecd10415884163fb644fce610882e7ed8190c6e205ab4e68"),
	hex2array!("f8fb25eae1246c013b662bee0cc69a8724f26a0e4c42a6a749babb8296c85939"),
	hex2array!("6cb0b18c01ba0765378d1e3d49aaac89f45a9e33cfc71c76386c225549f1151a"),
	hex2array!("dac4e419ea4348088c5357472eddb96520ac29e116ae442e58f3b328923bab3d"),
	hex2array!("a82719fd213c936b5fcdc786581472cac04053ecc00da61280a1fa022e4e8d41"),
	hex2array!("1c2c9118870d5a5ccf9707413e0b42c274a10e9c725648f94a0829ece0f8d106"),
	hex2array!("5a5b6a4151c0b03691398af54d7980b98c5ac91445e74953b713657cb18ca80b"),
	hex2array!("3ca7117dff8cd73e8a0d023e7bf78c5f637846227b4092abe000fbf21b57de1e"),
	hex2array!("b8d4d2663b66ae82aee2f341fed2977ab875d93cebf874b99ef2773615cc9148"),
	hex2array!("4444c14c65f94af110655bc47f041dbef0daa75fcde6a41520c85ecb4e7f9903"),
	hex2array!("a295162e8ce0465874479d07762edc7d4dc1f8b919d6ed9bd33f0b3b2f2d0718"),
	hex2array!("c49b3886738a2d5af650e22a167b9b2f6ccc61099ea489d4010679388dfe0b76"),
	hex2array!("1a21a884daffb6d87caa16506269117a501bb0cee5a7fc40809aa5265421ef0b"),
	hex2array!("1415b4f09ff5b563cd63663dacb83e6a8c0d143e433efcb80939f07d6c88734d"),
	hex2array!("9cb1f1e93d1a2dff01426150ed29c3bc3e927d59986a54f524756a9655d7b14d"),
	hex2array!("e861da7a180d7f5f2c8cde5c547728604b905ff0f180d4d8d6fe4b6552660967"),
	hex2array!("74ff08c51077acd324976370a3d661131c1e9a22d34dada43706dccbbc437741"),
	hex2array!("36cfa0144cb0461fd2a4bcb8aa668c5c4868d7528351588d08f514af41bb6345"),
	hex2array!("2e7eaf519fc0931f117b1bd56b4eba7d20c787d97cb409476c53da69a18c5633"),
	hex2array!("50a475ed252b3c06bdf5fa448816fb1e44b770204e49c76ce5a6db7a7ac36e49"),
];

/// Update provider count on chains that ran pallet pre-release (#8353)
pub struct ExtendedProviderMigration<T>(PhantomData<T>);

impl<T: pallet_delegated_staking::Config> OnRuntimeUpgrade for ExtendedProviderMigration<T>
where
	T::AccountId: From<AccountId32>,
{
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// Increase provider count of all bonds created previously
		let weight = pallet_delegated_staking::migration::unversioned::IncProviderMigration::<T>::on_runtime_upgrade();

		let mut count = 0;

		// Decrease provider count on bonds created on new runtime version
		for account in MIGRATED {
			let id = AccountId::new(account).into();
			let prov = frame_system::Pallet::<T>::providers(&id);
			if prov == 3 {
				let _ = frame_system::Pallet::<T>::dec_providers(&id);
			} else {
				log::error!("Missmatch of {prov} detected: {id:?}");
			}
			count += 1;
		}

		// Each Provider update has results in one read and write
		weight + T::DbWeight::get().reads_writes(count as u64, count as u64)
	}
}
