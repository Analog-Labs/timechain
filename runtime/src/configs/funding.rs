//! On-chain funding configuration

use polkadot_sdk::*;

#[cfg(feature = "testnet")]
use frame_system::{EnsureRoot, EnsureWithSuccess};

#[cfg(feature = "testnet")]
use frame_support::traits::{
	tokens::{PayFromAccount, UnityAssetBalanceConversion},
	EitherOfDiverse,
};
use frame_support::{parameter_types, PalletId};

#[cfg(feature = "testnet")]
use sp_runtime::{traits::IdentityLookup, Percent, Permill};

// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>

#[cfg(feature = "testnet")]
use time_primitives::BlockNumber;

// Local module imports
#[cfg(not(feature = "testnet"))]
use crate::EnsureRootOrHalfTechnical;
#[cfg(feature = "testnet")]
use crate::{deposit, AccountId, Balance, Balances, TechnicalCollective, Treasury, ANLOG, DAYS};
use crate::{main_or_test, weights, ExistentialDeposit, Runtime, RuntimeEvent, Vesting};

#[cfg(feature = "testnet")]
parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 1 * ANLOG;
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(1);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * ANLOG;
	pub const DataDepositPerByte: Balance = deposit(0,1);
	pub const TreasuryPalletId: PalletId = PalletId(*b"timetrsy");
	pub const MaximumReasonLength: u32 = 300;
	pub const MaxApprovals: u32 = 100;
	pub const MaxBalance: Balance = Balance::MAX;
	pub const SpendPayoutPeriod: BlockNumber = 30 * DAYS;
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

#[cfg(feature = "testnet")]
impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<AccountId, TechnicalCollective, 1, 2>,
	>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type MaxApprovals = MaxApprovals;
	type SpendOrigin = EnsureWithSuccess<EnsureRoot<AccountId>, AccountId, MaxBalance>;
	type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = UnityAssetBalanceConversion;
	type PayoutPeriod = SpendPayoutPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub RawPrefix: &'static [u8] = main_or_test!(b"Airdrop ANLOG to the Timechain account: ", b"Airdrop TANLOG to the Testnet account: ");
	pub LaunchId: PalletId = PalletId(*b"timelnch");
}

impl pallet_airdrop::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type RawPrefix = RawPrefix;
	type MinimumBalance = ExistentialDeposit;
	type WeightInfo = weights::pallet_airdrop::WeightInfo<Runtime>;
}

#[cfg(not(feature = "testnet"))]
impl pallet_launch::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = LaunchId;
	type MinimumDeposit = ExistentialDeposit;
	type LaunchAdmin = EnsureRootOrHalfTechnical;
	type WeightInfo = weights::pallet_launch::WeightInfo<Runtime>;
}
