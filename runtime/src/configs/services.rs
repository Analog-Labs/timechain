// === Low level on chain services ===

use polkadot_sdk::*;

use frame_support::{
	parameter_types,
	traits::{fungible::HoldConsideration, ConstU32, EqualPrivilegeOnly, LinearStoragePrice},
	weights::Weight,
};
use frame_system::EnsureRoot;

use sp_runtime::{traits::Verify, Perbill};

use pallet_identity::legacy::IdentityInfo;

use time_primitives::{Signature, ANLOG};
// Local module imports
use crate::{
	deposit, AccountId, Balance, Balances, EnsureRootOrHalfTechnical, OriginCaller, Preimage,
	Runtime, RuntimeBlockWeights, RuntimeCall, RuntimeEvent, RuntimeHoldReason, RuntimeOrigin,
	Treasury, DAYS,
};

parameter_types! {
	// difference of 26 bytes on-chain for the registration and 9 bytes on-chain for the identity
	// information, already accounted for by the byte deposit
	pub const BasicDeposit: Balance = deposit(1, 17);
	pub const ByteDeposit: Balance = deposit(0, 1);
	pub const SubAccountDeposit: Balance = 2 * ANLOG;   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

#[cfg(feature = "testnet")]
impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = EnsureRootOrHalfTechnical;
	type RegistrarOrigin = EnsureRootOrHalfTechnical;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as Verify>::Signer;
	type UsernameAuthorityOrigin = EnsureRoot<Self::AccountId>;
	type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
	type MaxSuffixLength = ConstU32<7>;
	type MaxUsernameLength = ConstU32<32>;
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

#[cfg(feature = "testnet")]
parameter_types! {
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
	/// TODO: Select good value
	pub const StorageBaseDeposit: Balance = 1 * ANLOG;
	pub const StorageByteDeposit: Balance = deposit(0,1);
}

#[cfg(feature = "testnet")]
impl pallet_preimage::Config for Runtime {
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<StorageBaseDeposit, StorageByteDeposit, Balance>,
	>;
}

parameter_types! {
	   pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
			   RuntimeBlockWeights::get().max_block;
}

#[cfg(feature = "testnet")]
impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	#[cfg(feature = "runtime-benchmarks")]
	type MaxScheduledPerBlock = ConstU32<512>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
}
