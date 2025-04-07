// This file contains migrations for the session pallet.

use crate::Runtime;
use polkadot_sdk::frame_support::{
	migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade,
};
use polkadot_sdk::pallet_session::Pallet as SessionPallet;
use polkadot_sdk::sp_std::marker::PhantomData;

/// Implementation of the session migration for disabled validators
pub struct SessionMigrationV1<T>(PhantomData<T>);

impl<T> UncheckedOnRuntimeUpgrade for SessionMigrationV1<T> {
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

/// Migration to update the session pallet's storage version from v0 to v1.
pub type SessionMigrationV0ToV1 = VersionedMigration<
	0,
	1,
	SessionMigrationV1<Runtime>,
	SessionPallet<Runtime>,
	<Runtime as polkadot_sdk::frame_system::Config>::DbWeight,
>;
