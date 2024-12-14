///! Helpers to handle runtime variants

// Safety check to preserve assumption via code
#[cfg(all(feature = "develop", not(feature = "testnet")))]
compile_error!("Runtime variant \"develop\" expects \"testnet\" feature to be set as well.");

/// Mainnet runtime variant string
#[cfg(not(any(feature = "testnet", feature = "develop")))]
const RUNTIME_VARIANT: &str = "mainnet";

/// Testnet runtime variant string
#[cfg(all(feature = "testnet", not(feature = "develop")))]
const RUNTIME_VARIANT: &str = "testnet";

/// Develop runtime variant string
#[cfg(feature = "develop")]
const RUNTIME_VARIANT: &str = "develop";

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
