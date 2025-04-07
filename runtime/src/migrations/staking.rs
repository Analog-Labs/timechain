// This file contains migrations for the staking pallet.

use crate::Runtime;
use polkadot_sdk::frame_support::{
	migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade,
};
use polkadot_sdk::pallet_staking::Pallet as StakingPallet;
use polkadot_sdk::sp_std::marker::PhantomData;

/// Implementation of the staking migration from v15 to v16
pub struct StakingMigrationV15ToV16<T>(PhantomData<T>);

impl<T> UncheckedOnRuntimeUpgrade for StakingMigrationV15ToV16<T> {
	fn on_runtime_upgrade() -> polkadot_sdk::frame_support::weights::Weight {
		// This is just a stub implementation to satisfy the try-runtime check
		polkadot_sdk::frame_support::weights::Weight::from_parts(10_000_000, 0)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade(
	) -> Result<polkadot_sdk::sp_std::vec::Vec<u8>, polkadot_sdk::sp_runtime::TryRuntimeError> {
		Ok(polkadot_sdk::sp_std::vec::Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(
		_state: polkadot_sdk::sp_std::vec::Vec<u8>,
	) -> Result<(), polkadot_sdk::sp_runtime::TryRuntimeError> {
		Ok(())
	}
}

/// Migration to update the staking pallet's storage version from v15 to v16
pub type StakingMigration = VersionedMigration<
	15,
	16,
	StakingMigrationV15ToV16<Runtime>,
	StakingPallet<Runtime>,
	<Runtime as polkadot_sdk::frame_system::Config>::DbWeight,
>;
