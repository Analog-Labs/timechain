use crate::{
	self as pallet_assets_bridge, AssetTeleporter, BalanceOf, BeneficiaryOf, Error, NetworkDataOf,
	NetworkIdOf,
};
use core::marker::PhantomData;

use polkadot_sdk::{
	frame_support, frame_system, pallet_balances, pallet_treasury, sp_core, sp_io, sp_runtime,
	sp_std,
};

use frame_support::traits::{
	tokens::{ConversionFromAssetBalance, Pay, PaymentStatus},
	OnInitialize,
};
use frame_support::PalletId;
use frame_support::{derive_impl, ensure};

use sp_core::{ConstU128, ConstU16, ConstU32, ConstU64};
use sp_runtime::{
	traits::{parameter_types, Get, IdentifyAccount, IdentityLookup, Verify},
	AccountId32, BuildStorage, DispatchResult, MultiSignature, Permill,
};
use sp_std::cell::RefCell;
use sp_std::collections::btree_map::BTreeMap;

use time_primitives::{
	Address, Balance, ElectionsInterface, GmpMessage, MembersInterface, NetworkId,
	NetworksInterface, PeerId, PublicKey, ShardsInterface, U256,
};

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

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

pub struct MockMembers;

impl MembersInterface for MockMembers {
	fn member_stake(_: &AccountId) -> Balance {
		0u128
	}
	fn member_peer_id(_: &AccountId) -> Option<PeerId> {
		None
	}
	fn member_public_key(_account: &AccountId) -> Option<PublicKey> {
		Some(sp_runtime::MultiSigner::Sr25519(sp_core::sr25519::Public::from_raw([0u8; 32])))
	}
	fn is_member_online(_: &AccountId) -> bool {
		true
	}
	fn total_stake() -> u128 {
		0u128
	}
	fn transfer_stake(_: &AccountId, _: &AccountId, _: Balance) -> DispatchResult {
		Ok(())
	}
	fn unstake_member(_account: &AccountId) {}
	fn is_member_registered(_account: &AccountId) -> bool {
		true
	}
}

pub struct MockElections;

impl ElectionsInterface for MockElections {
	type MaxElectionsPerBlock = ConstU32<10>;
	fn shard_offline(_: NetworkId, _: Vec<AccountId>) {}
	fn member_online(member: &AccountId, network: NetworkId) {
		Shards::member_online(member, network)
	}
	fn member_offline(member: &AccountId, network: NetworkId) {
		Shards::member_offline(member, network)
	}
}

impl AssetTeleporter<Test> for Tasks {
	fn handle_register(
		network_id: NetworkIdOf<Test>,
		_data: &mut NetworkDataOf<Test>,
	) -> DispatchResult {
		ensure!(
			network_id.ne(&<Test as pallet_networks::Config>::TimechainNetworkId::get() as &u16),
			Error::<Test>::NetworkAlreadyExists
		);

		Ok(())
	}

	fn handle_teleport(
		network_id: NetworkIdOf<Test>,
		details: &mut NetworkDataOf<Test>,
		beneficiary: BeneficiaryOf<Test>,
		amount: BalanceOf<Test>,
	) -> DispatchResult {
		// TODO better to store somewhere
		let src: Address = Bridge::account_id().into();
		// see struct TeleportCommand in the teleport-tokens/BasicERC20.sol
		// TODO refactor with alloy
		let teleport_command =
			[&src[..], &beneficiary[..], &U256::from(amount).to_big_endian()[..]]
				.concat()
				.to_vec();
		let msg = GmpMessage {
			src_network: <Test as pallet_networks::Config>::TimechainNetworkId::get(),
			dest_network: network_id,
			src,
			// GMP backend truncates this to AccountId20
			dest: details.1,
			nonce: details.0,
			// must be sufficient to exec OnGmpRecieved
			gas_limit: 100_000u128,
			// ussually used for chronicles refund
			gas_cost: 0u128,
			// calldata for our OnGmpRecieved
			bytes: teleport_command,
		};

		// Push GMP message to gateway ops queue
		Tasks::push_gmp_message(msg);

		Ok(())
	}
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub struct Test {
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Tasks: pallet_tasks::{Pallet, Call, Storage, Event<T>},
		Shards: pallet_shards::{Pallet, Call, Storage, Event<T>},
		Members: pallet_members,
		Elections: pallet_elections,
		Treasury: pallet_treasury,
		Networks: pallet_networks,
		Bridge: pallet_assets_bridge,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeTask = RuntimeTask;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
}

impl pallet_assets_bridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type PalletId = BridgePalletId;
	type Currency = pallet_balances::Pallet<Test>;
	type FeeDestination = Treasury;
	type NetworkId = u16;
	type NetworkData = (u64, Address);
	type Beneficiary = Address;
	type Teleporter = Tasks;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
}

thread_local! {
	pub static PAID: RefCell<BTreeMap<(u128, u32), u128>> = const { RefCell::new(BTreeMap::new()) };
	pub static STATUS: RefCell<BTreeMap<u64, PaymentStatus>> = const { RefCell::new(BTreeMap::new()) };
	pub static LAST_ID: RefCell<u64> = const { RefCell::new(0u64) };
}

/// set status for a given payment id
#[allow(dead_code)]
fn set_status(id: u64, s: PaymentStatus) {
	STATUS.with(|m| m.borrow_mut().insert(id, s));
}

pub struct TestPay;
impl Pay for TestPay {
	type Beneficiary = u128;
	type Balance = u128;
	type Id = u64;
	type AssetKind = u32;
	type Error = ();

	fn pay(
		who: &Self::Beneficiary,
		asset_kind: Self::AssetKind,
		amount: Self::Balance,
	) -> Result<Self::Id, Self::Error> {
		PAID.with(|paid| *paid.borrow_mut().entry((*who, asset_kind)).or_default() += amount);
		Ok(LAST_ID.with(|lid| {
			let x = *lid.borrow();
			lid.replace(x + 1);
			x
		}))
	}
	fn check_payment(id: Self::Id) -> PaymentStatus {
		STATUS.with(|s| s.borrow().get(&id).cloned().unwrap_or(PaymentStatus::Unknown))
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(_: &Self::Beneficiary, _: Self::AssetKind, _: Self::Balance) {}
	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_concluded(id: Self::Id) {
		set_status(id, PaymentStatus::Failure)
	}
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	// 5EYCAe5ijiYfyeZ2JJCGq56LmPyNRAKzpG4QkoQkkQNB5e6Z
	pub TreasuryAccount: AccountId = Treasury::account_id();
	pub const SpendPayoutPeriod: u64 = 5;
	pub const BridgePalletId: PalletId = PalletId(*b"py/bridg");
	pub BridgeAccount: AccountId = Bridge::account_id();
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

impl pallet_members::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Elections = MockElections;
	type Shards = Shards;
	type MinStake = ConstU128<5>;
	type HeartbeatTimeout = ConstU64<10>;
	type MaxTimeoutsPerBlock = ConstU32<100>;
}

impl pallet_elections::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type Shards = Shards;
	type Members = Members;
	type Networks = MockNetworks;
	type MaxElectionsPerBlock = ConstU32<10>;
}

impl pallet_shards::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type Tasks = pallet_tasks::Pallet<Test>;
	type Members = MockMembers;
	type Elections = Elections;
	type DkgTimeout = ConstU64<10>;
}

impl pallet_networks::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type Tasks = Tasks;
	type TimechainNetworkId = ConstU16<1000>;
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

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		account: AccountId,
		_nonce: u32,
	) -> Option<(
		RuntimeCall,
		<UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
	)> {
		Some((call, (account, (), ())))
	}
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let _ = env_logger::try_init();
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(acc(0).into(), 10_000_000_000), (acc(1).into(), 20_000_000_000)],
	}
	.assimilate_storage(&mut storage)
	.unwrap();
	let mut ext: sp_io::TestExternalities = storage.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn acc(acc_num: u8) -> AccountId32 {
	AccountId32::new([acc_num; 32])
}

pub fn roll(n: u64) {
	for _ in 0..n {
		next_block();
	}
}

fn next_block() {
	let mut now = System::block_number();
	now += 1;
	System::set_block_number(now);
	Shards::on_initialize(now);
	Tasks::on_initialize(now);
}
