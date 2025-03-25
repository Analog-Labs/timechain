//! Configuration for the pallet-revive module.
//! This module is only included in the testnet runtime.

use crate::*;
use frame_support::{
    parameter_types,
    traits::{ConstU32, ConstU64},
};
use pallet_revive::Config;
use parity_scale_codec::Encode;

parameter_types! {
    // Chain ID for Ethereum compatibility
    pub const ChainId: u64 = 2046; // Westend-aligned chain ID
    
    // Gas limit for PolkaVM execution
    pub const BlockGasLimit: u64 = 15_000_000;
    
    // Maximum size of code that can be deployed
    pub const MaxCodeLen: u32 = 128 * 1024; // 128 KB
    
    // Maximum amount of static memory a contract can use
    pub const RuntimeMemory: u32 = 16 * 1024 * 1024; // 16 MB
    
    // Maximum size of immutable data that can be stored
    pub const PVFMemory: u32 = 1024 * 1024; // 1 MB
    
    // Deposit per byte for storing contract code
    pub const DepositPerByte: Balance = 100 * MICROUNIT;
    
    // Deposit per storage item
    pub const DepositPerItem: Balance = 1 * MILLIUNIT;
    
    // Percentage of code hash deposit that is locked
    pub const CodeHashLockupDepositPercent: u32 = 30;
}

/// Configure the pallet-revive module.
impl Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Time = Timestamp;
    type RuntimeCall = RuntimeCall;
    type RuntimeHoldReason = RuntimeHoldReason;
    type CallFilter = frame_support::traits::Everything;
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    type WeightInfo = pallet_revive::weights::SubstrateWeight<Runtime>;
    type ChainExtension = (); // No chain extension
    type DepositPerByte = DepositPerByte;
    type DepositPerItem = DepositPerItem;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type AddressGenerator = pallet_revive::DefaultAddressGenerator;
    type MaxCodeLen = MaxCodeLen;
    type UnsafeUnstableInterface = ConstU32<0>; // Disable unsafe interfaces
    type UploadOrigin = frame_system::EnsureRoot<AccountId>;
    type InstantiateOrigin = frame_system::EnsureSigned<AccountId>;
    type Migrations = (); // No migrations
    type Debug = (); // No debugging
    type Xcm = (); // No XCM integration
    type RuntimeMemory = RuntimeMemory;
    type PVFMemory = PVFMemory;
}
