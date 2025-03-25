//! The runtime configuration by sections

pub mod consensus;
pub mod core;
#[cfg(not(feature = "testnet"))]
pub mod funding;
pub mod governance;
#[cfg(feature = "testnet")]
pub mod revive;
#[cfg(feature = "testnet")]
pub mod services;
pub mod staking;
pub mod tokenomics;
pub mod utilities;

pub mod bridge;
#[cfg(feature = "testnet")]
pub mod custom;
