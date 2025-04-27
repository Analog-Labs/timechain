use crate::apis;

use polkadot_sdk::*;

use sp_version::{Cow, RuntimeVersion};

/// Mainnet runtime version
#[cfg(not(any(feature = "testnet", feature = "develop")))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("analog-timechain"),
	impl_name: Cow::Borrowed("analog-timechain"),
	authoring_version: 0,
	spec_version: 30,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

/// Staging runtime version.
#[cfg(all(not(feature = "testnet"), feature = "develop"))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("analog-staging"),
	impl_name: Cow::Borrowed("analog-staging"),
	authoring_version: 0,
	spec_version: 30,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

/// Testnet runtime version.
#[cfg(all(feature = "testnet", not(feature = "develop")))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("analog-testnet"),
	impl_name: Cow::Borrowed("analog-testnet"),
	authoring_version: 0,
	spec_version: 30,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

/// Development runtime version.
#[cfg(all(feature = "testnet", feature = "develop"))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("analog-develop"),
	impl_name: Cow::Borrowed("analog-develop"),
	authoring_version: 0,
	spec_version: 30,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};
