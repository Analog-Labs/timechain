use polkadot_sdk::*;

use frame_support::parameter_types;

// Local module imports
use crate::{deposit, Balance, Balances, Runtime, RuntimeCall, RuntimeEvent};

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

// TODO: update deposits to sensible values
parameter_types! {
	/// Base deposit required for storing a multisig execution, covering the cost of a single storage item.
	// One storage item; key size is 32; value is size 4+4(block number)+16(balance)+32(account ID) bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u16 = 100;
}

impl bridge_multisig::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = bridge_multisig::weights::SubstrateWeight<Runtime>;
}
