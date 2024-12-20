//! Automatically generated runtime weight selected based on features flags

/// Mainnet profile runtime weights (default)
#[cfg(not(any(feature = "testnet", feature = "develop")))]
mod mainnet;

#[cfg(not(any(feature = "testnet", feature = "develop")))]
pub use mainnet::*;

/// Testnet profile runtime weights
#[cfg(all(feature = "testnet", not(feature = "develop")))]
mod testnet;

#[cfg(all(feature = "testnet", not(feature = "develop")))]
pub use testnet::*;

/// Develop profile runtime weights
#[cfg(feature = "develop")]
mod develop;

#[cfg(feature = "develop")]
pub use develop::*;
