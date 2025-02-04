use polkadot_sdk::*;

use frame_support::parameter_types;

// Local module imports
use crate::{Balances, Runtime, RuntimeCall, RuntimeEvent};

pub type NetworkId = u32;

#[cfg(not(feature = "testnet"))]
use super::governance::EnsureRootOrHalfTechnical;
#[cfg(feature = "testnet")]
use super::governance::EnsureRootOrTechnicalMember;

#[cfg(not(feature = "testnet"))]
/// Default admin origin for all chronicle related pallets
type ChronicleAdmin = EnsureRootOrHalfTechnical;

#[cfg(feature = "testnet")]
/// Development admin origin for all chronicle related pallets
type ChronicleAdmin = EnsureRootOrTechnicalMember;

impl eth_bridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type NetworkId = NetworkId;
	type PeerId = eth_bridge::offchain::crypto::TestAuthId;
	type WeightInfo = eth_bridge::weights::SubstrateWeight<Runtime>;
	type AdminOrigin = ChronicleAdmin;
}

parameter_types! {
	pub const DepositBase: u64 = 1;
	pub const DepositFactor: u64 = 1;
	pub const MaxSignatories: u16 = 100;
}

impl bridge_multisig::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = ();
}
