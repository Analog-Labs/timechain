use polkadot_sdk::*;

use frame_support::parameter_types;

// Local module imports
use crate::{deposit, weights, Balance, Balances, DefaultAdminOrigin, Runtime, RuntimeCall, RuntimeEvent};

pub type NetworkId = u32;

impl eth_bridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type NetworkId = NetworkId;
	type PeerId = eth_bridge::offchain::crypto::TestAuthId;
	type WeightInfo = weights::eth_bridge::WeightInfo<Runtime>;
	type AdminOrigin = DefaultAdminOrigin;
}

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
	type WeightInfo = weights::bridge_multisig::WeightInfo<Runtime>;
}
