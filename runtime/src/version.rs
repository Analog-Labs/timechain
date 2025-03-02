use crate::apis;

use polkadot_sdk::*;

use sp_runtime::create_runtime_str;
use sp_version::RuntimeVersion;

/// Mainnet runtime version
#[cfg(not(any(feature = "testnet", feature = "develop")))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-timechain"),
	impl_name: create_runtime_str!("analog-timechain"),
	authoring_version: 0,
	spec_version: 19,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// Staging runtime version.
#[cfg(all(not(feature = "testnet"), feature = "develop"))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-staging"),
	impl_name: create_runtime_str!("analog-staging"),
	authoring_version: 0,
	spec_version: 19,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// Testnet runtime version.
#[cfg(all(feature = "testnet", not(feature = "develop")))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-testnet"),
	impl_name: create_runtime_str!("analog-testnet"),
	authoring_version: 0,
	spec_version: 19,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// Development runtime version.
#[cfg(all(feature = "testnet", feature = "develop"))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-develop"),
	impl_name: create_runtime_str!("analog-develop"),
	authoring_version: 0,
	spec_version: 19,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};
