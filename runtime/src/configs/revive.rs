//! Configuration for the pallet-revive module.
//! This module is only included in the testnet runtime.

use crate::*;
use frame_support::{parameter_types, traits::Everything};
use pallet_revive::Config;
use sp_runtime::{traits::ConstU32, Perbill};
use time_primitives::{MICROANLOG, MILLIANLOG};

parameter_types! {
	// Chain ID for Ethereum compatibility
	pub const ChainId: u64 = 2046; // TODO UPDATE

	// Gas limit for PolkaVM execution
	pub const BlockGasLimit: u64 = 15_000_000;

	// Deposit per byte for storing contract code
	pub const DepositPerByte: Balance = 100 * MICROANLOG; // 100 micro units

	// Deposit per storage item
	pub const DepositPerItem: Balance = MILLIANLOG; // 1 milli unit

	// Percentage of code hash deposit that is locked
	pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
}

/// Configure the pallet-revive module.
impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Time = Timestamp;
	type RuntimeCall = RuntimeCall;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CallFilter = Everything;
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = pallet_revive::weights::SubstrateWeight<Runtime>;
	type ChainExtension = (); // No chain extension
	type DepositPerByte = DepositPerByte;
	type DepositPerItem = DepositPerItem;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type AddressGenerator = pallet_revive::DefaultAddressGenerator;
	type MaxCodeLen = ConstU32<{ 123 * 1024 }>; // 123 KB
	type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
	type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
	type UnsafeUnstableInterface = frame_support::traits::ConstBool<false>; // Disable unsafe interfaces
	type UploadOrigin = frame_system::EnsureSigned<AccountId>;
	type InstantiateOrigin = frame_system::EnsureSigned<AccountId>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Migrations = ();
	#[cfg(feature = "runtime-benchmarks")]
	type Migrations = pallet_revive::migration::codegen::BenchMigrations;
	type Debug = (); // No debugging
	type Xcm = (); // No XCM integration
}
