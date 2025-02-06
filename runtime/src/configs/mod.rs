//! The runtime configuration by sections

pub mod consensus;
pub mod core;
pub mod funding;
pub mod governance;
#[cfg(feature = "testnet")]
pub mod services;
pub mod staking;
pub mod tokenomics;
pub mod utilities;

#[cfg(feature = "testnet")]
pub mod bridge;
#[cfg(feature = "testnet")]
pub mod custom;
