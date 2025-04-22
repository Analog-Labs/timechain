//! Configuration for the pallet-revive module.
//! This module is only included in the testnet runtime.

use crate::*;
use frame_support::{parameter_types, traits::Nothing};
use pallet_revive::Config;
use sp_runtime::{
	traits::{ConstBool, ConstU32, ConstU64},
	Perbill,
};
use time_primitives::{MICROANLOG, MILLIANLOG};

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
}

impl Config for Runtime {
	type Time = Timestamp;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type CallFilter = Nothing;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = pallet_revive::weights::SubstrateWeight<Runtime>;
	type ChainExtension = ();
	type AddressMapper = pallet_revive::AccountId32Mapper<Self>;
	type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
	type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type UploadOrigin = frame_system::EnsureSigned<AccountId>;
	type InstantiateOrigin = frame_system::EnsureSigned<AccountId>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type Xcm = ();
	type ChainId = ConstU64<12_850>;
	type NativeToEthRatio = ConstU32<1_000_000>; // 10^(18 - 12) Eth is 10^18, Native is 10^12.
	type EthGasEncoder = ();
	type FindAuthor = <Runtime as pallet_authorship::Config>::FindAuthor;
}

impl TryFrom<RuntimeCall> for pallet_revive::Call<Runtime> {
	type Error = ();

	fn try_from(value: RuntimeCall) -> Result<Self, Self::Error> {
		match value {
			RuntimeCall::Revive(call) => Ok(call),
			_ => Err(()),
		}
	}
}
