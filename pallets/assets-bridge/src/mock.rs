use crate as pallet_assets_bridge;

use polkadot_sdk::*;

use core::marker::PhantomData;
use frame_support::traits::{Get,
	tokens::{ConversionFromAssetBalance, Pay, PaymentStatus},
	OnInitialize,
};
use frame_support::{derive_impl, PalletId};
use sp_core::{ConstU128, ConstU32, ConstU64};
use sp_runtime::{
	traits::{parameter_types, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, DispatchError, MultiSignature, Permill,
};

use time_primitives::{Address, ElectionsInterface, NetworkId, NetworksInterface, PublicKey};

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

pub fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

pub struct MockNetworks;

impl NetworksInterface for MockNetworks {
	fn gateway(_network: NetworkId) -> Option<Address> {
		Some([0; 32])
	}
	fn get_networks() -> Vec<NetworkId> {
		vec![0]
	}
	fn next_batch_size(_network: NetworkId, _block_height: u64) -> u32 {
		5
	}
	fn batch_gas_limit(_network: NetworkId) -> u128 {
		10
	}
	fn shard_task_limit(_network: NetworkId) -> u32 {
		10
	}
	fn shard_size(_network: NetworkId) -> u16 {
		3
	}
	fn shard_threshold(_network: NetworkId) -> u16 {
		2
	}
}

frame_support::construct_runtime!(
	pub struct Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Treasury: pallet_treasury,
		Tasks: pallet_tasks::{Pallet, Call, Storage, Event<T>},
		Shards: pallet_shards::{Pallet, Call, Storage, Event<T>},
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeTask = RuntimeTask;
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	// 5EYCAe5ijiYfyeZ2JJCGq56LmPyNRAKzpG4QkoQkkQNB5e6Z
	pub TreasuryAccount: AccountId = Treasury::account_id();
	pub const SpendPayoutPeriod: u64 = 5;
}
pub struct TestSpendOrigin;
impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for TestSpendOrigin {
	type Success = u128;
	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		Result::<frame_system::RawOrigin<_>, RuntimeOrigin>::from(o).and_then(|o| match o {
			frame_system::RawOrigin::Root => Ok(u128::MAX),
			r => Err(RuntimeOrigin::from(r)),
		})
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

pub struct MulBy<N>(PhantomData<N>);
impl<N: Get<u128>> ConversionFromAssetBalance<u128, u32, u128> for MulBy<N> {
	type Error = ();
	fn from_asset_balance(balance: u128, _asset_id: u32) -> Result<u128, Self::Error> {
		balance.checked_mul(N::get()).ok_or(())
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(_: u32) {}
}

impl pallet_treasury::Config for Test {
	type PalletId = TreasuryPalletId;
	type Currency = pallet_balances::Pallet<Test>;
	type RejectOrigin = frame_system::EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = ConstU64<2>;
	type Burn = Burn;
	type BurnDestination = (); // Just gets burned.
	type WeightInfo = ();
	type SpendFunds = ();
	type MaxApprovals = ConstU32<100>;
	type SpendOrigin = TestSpendOrigin;
	type AssetKind = u32;
	type Beneficiary = u128;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = TestPay;
	type BalanceConverter = MulBy<ConstU128<2>>;
	type PayoutPeriod = SpendPayoutPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_shards::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type Tasks = Tasks;
	type Members = ();
	type Elections = ();
	type DkgTimeout = ConstU64<10>;
}

impl pallet_tasks::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type Shards = Shards;
	type Networks = MockNetworks;
	type MaxTasksPerBlock = ConstU32<3>;
	type MaxBatchesPerBlock = ConstU32<4>;
}

pub fn acc_pub(acc_num: u8) -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([acc_num; 32])
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	// pallet_balances::GenesisConfig::<Test> {
	// 	balances: vec![(acc_pub(1).into(), 10_000_000_000), (acc_pub(2).into(), 20_000_000_000)],
	// }
	// .assimilate_storage(&mut storage)
	// .unwrap();
	let mut ext: sp_io::TestExternalities = storage.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
