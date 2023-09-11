/// Integration tests
use crate::*;
use frame_support::traits::WhitelistedStorageKeys;
use frame_support::{assert_ok, traits::OnInitialize};
use frame_system::RawOrigin;
use pallet_tasks::TaskPhaseState;
use sp_core::hexdisplay::HexDisplay;
use std::collections::HashSet;
use time_primitives::{
	AccountId, Function, Network, PublicKey, ShardStatus, ShardsInterface, TaskDescriptorParams,
	TaskPhase, TasksInterface,
};

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}
fn acc_pub(acc_num: u8) -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([acc_num; 32])
}

const A: [u8; 32] = [1u8; 32];
const B: [u8; 32] = [2u8; 32];
const C: [u8; 32] = [3u8; 32];

// Build genesis storage according to the mock runtime.
fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(acc_pub(1).into(), 10_000_000_000), (acc_pub(2).into(), 20_000_000_000)],
	}
	.assimilate_storage(&mut storage)
	.unwrap();
	let mut ext: sp_io::TestExternalities = storage.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// To from `now` to block `n`.
fn roll_to(n: u32) {
	let now = System::block_number();
	for i in now + 1..=n {
		System::set_block_number(i);
		Tasks::on_initialize(i);
	}
}

#[test]
fn write_phase_timeout_reassigns_task() {
	let task_id = 0;
	let a: AccountId = A.into();
	let b: AccountId = B.into();
	let c: AccountId = C.into();
	let shard = [a.clone(), b.clone(), c.clone()].to_vec();
	new_test_ext().execute_with(|| {
		assert_ok!(Members::register_member(
			RawOrigin::Signed(a.clone()).into(),
			Network::Ethereum,
			pubkey_from_bytes(A),
			A,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(b.clone()).into(),
			Network::Ethereum,
			pubkey_from_bytes(B),
			B,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(c.clone()).into(),
			Network::Ethereum,
			pubkey_from_bytes(C),
			C,
		));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed(a.clone()).into(),
			TaskDescriptorParams {
				network: Network::Ethereum,
				function: Function::EvmCall {
					address: Default::default(),
					function_signature: Default::default(),
					input: Default::default(),
					amount: 0,
				},
				cycle: 1,
				start: 0,
				period: 0,
				hash: "".to_string(),
			}
		));
		Shards::create_shard(Network::Ethereum, shard, 1);
		<pallet_shards::ShardState<Runtime>>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_eq!(
			<TaskPhaseState<Runtime>>::get(task_id),
			TaskPhase::Write(pubkey_from_bytes(A), 1)
		);
		roll_to(10);
		assert_eq!(
			<TaskPhaseState<Runtime>>::get(task_id),
			TaskPhase::Write(pubkey_from_bytes(A), 1)
		);
		roll_to(11);
		assert_eq!(
			<TaskPhaseState<Runtime>>::get(task_id),
			TaskPhase::Write(pubkey_from_bytes(B), 11)
		);
	});
}

#[test]
fn check_whitelist() {
	let whitelist: HashSet<String> = AllPalletsWithSystem::whitelisted_storage_keys()
		.iter()
		.map(|e| HexDisplay::from(&e.key).to_string())
		.collect();

	// Block Number
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac"));
	// Total Issuance
	assert!(whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80"));
	// Execution Phase
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a"));
	// Event Count
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850"));
	// System Events
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"));
}

#[test]
fn check_arithmetic() {
	let max_payout = 100u32;
	let session_active_validators = 4u8;
	let send_reward = Percent::from_percent(20) * max_payout;
	assert_eq!(send_reward, 20); // 20 percent of total reward
	let perc_div = 100u8.saturating_div(session_active_validators); // get division percentage for each validator
	let fraction = Percent::from_percent(perc_div);
	let share = fraction * send_reward;
	assert_eq!(share, 5); // 20 percent of total reward share of each validator.
}
