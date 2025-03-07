///! Helpers to handle runtime variants

/// Mainnet runtime variant string
#[cfg(not(any(feature = "testnet", feature = "develop")))]
pub const RUNTIME_VARIANT: &str = "mainnet";

/// Staging runtime variant string
#[cfg(all(not(feature = "testnet"), feature = "develop"))]
pub const RUNTIME_VARIANT: &str = "staging";

/// Testnet runtime variant string
#[cfg(all(feature = "testnet", not(feature = "develop")))]
pub const RUNTIME_VARIANT: &str = "testnet";

/// Develop runtime variant string
#[cfg(all(feature = "testnet", feature = "develop"))]
pub const RUNTIME_VARIANT: &str = "develop";

/// Macro to set a value (e.g. when using the `parameter_types` macro) based on
/// the the current runtime variant being build.
#[macro_export]
macro_rules! main_test_or_dev {
	($main:expr, $test:expr, $dev:expr) => {
		if cfg!(feature = "develop") {
			$dev
		} else if cfg!(feature = "testnet") {
			$test
		} else {
			$main
		}
	};
}

/// Macro to set value based on testnet feature flag
#[macro_export]
macro_rules! main_or_test {
	($main:expr, $test:expr) => {
		if cfg!(feature = "testnet") {
			$test
		} else {
			$main
		}
	};
}

/// Macro to set value based on develop feature flag
#[macro_export]
macro_rules! prod_or_dev {
	($prod:expr, $dev:expr) => {
		if cfg!(feature = "develop") {
			$dev
		} else {
			$prod
		}
	};
}
