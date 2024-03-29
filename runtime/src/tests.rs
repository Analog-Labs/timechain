/// Integration tests
use crate::*;
use frame_support::traits::WhitelistedStorageKeys;
use frame_support::{assert_ok, traits::OnInitialize};
use frame_system::RawOrigin;
use pallet_shards::ShardMembers;
use pallet_tasks::TaskSigner;
use sp_core::hexdisplay::HexDisplay;
use sp_core::Pair;
use std::collections::HashSet;
use time_primitives::{
	AccountId, ElectionsInterface, Function, MemberEvents, NetworkId, PublicKey, ShardStatus,
	ShardsInterface, TaskDescriptorParams, TasksInterface,
};

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}
fn acc_pub(acc_num: u8) -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([acc_num; 32])
}
fn get_peer_id(random_num: [u8; 32]) -> [u8; 32] {
	sp_core::ed25519::Pair::from_string(&format!("//{:?}", random_num), None)
		.unwrap()
		.public()
		.into()
}

const ETHEREUM: NetworkId = 0;
const A: [u8; 32] = [1u8; 32];
const B: [u8; 32] = [2u8; 32];
const C: [u8; 32] = [3u8; 32];
const D: [u8; 32] = [4u8; 32];
const E: [u8; 32] = [5u8; 32];

// FIXME: test assumes fixed shard size
const SHARD_SIZE: usize = 3;

// Build genesis storage according to the mock runtime.
fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	let mut balances = vec![];
	for i in 1..=(SHARD_SIZE * 3) {
		balances.push((acc_pub(i.try_into().unwrap()).into(), 100_000 * ANLOG));
	}
	pallet_balances::GenesisConfig::<Runtime> { balances }
		.assimilate_storage(&mut storage)
		.unwrap();
	pallet_elections::GenesisConfig::<Runtime>::default()
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
fn shard_not_stuck_in_committed_state() {
	let a: AccountId = A.into();
	let b: AccountId = B.into();
	let c: AccountId = C.into();
	//let d: AccountId = D.into();
	let first_shard = [c.clone(), b.clone(), a.clone()].to_vec();
	//let second_shard = [d.clone(), c.clone(), b.clone()].to_vec();
	new_test_ext().execute_with(|| {
		assert_ok!(Members::register_member(
			RawOrigin::Signed(a.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(A),
			get_peer_id(A),
			90_000 * ANLOG,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(b.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(B),
			get_peer_id(B),
			90_000 * ANLOG,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(c.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(C),
			get_peer_id(C),
			90_000 * ANLOG,
		));
		for (m, _) in ShardMembers::<Runtime>::iter_prefix(0) {
			assert!(first_shard.contains(&m));
		}
		// put shard in a committed state
		<pallet_shards::ShardState<Runtime>>::insert(0, ShardStatus::Committed);
		// then put all the members online
		for i in first_shard {
			Shards::member_online(&i, ETHEREUM);
		}
		assert_eq!(<pallet_shards::ShardState<Runtime>>::get(0).unwrap(), ShardStatus::Offline);
	});
}

#[test]
fn elections_chooses_top_members_by_stake() {
	let a: AccountId = A.into();
	let b: AccountId = B.into();
	let c: AccountId = C.into();
	let d: AccountId = D.into();
	let first_shard = [c.clone(), b.clone(), a.clone()].to_vec();
	let second_shard = [d.clone(), c.clone(), b.clone()].to_vec();
	new_test_ext().execute_with(|| {
		assert_ok!(Members::register_member(
			RawOrigin::Signed(a.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(A),
			get_peer_id(A),
			90_000 * ANLOG,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(b.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(B),
			get_peer_id(B),
			90_000 * ANLOG,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(c.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(C),
			get_peer_id(C),
			90_000 * ANLOG,
		));
		for (m, _) in ShardMembers::<Runtime>::iter_prefix(0) {
			assert!(first_shard.contains(&m));
		}
		assert_ok!(Members::register_member(
			RawOrigin::Signed(d.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(D),
			get_peer_id(D),
			90_001 * ANLOG,
		));
		Elections::shard_offline(ETHEREUM, vec![a.clone(), b.clone(), c.clone()]);
		for (m, _) in ShardMembers::<Runtime>::iter_prefix(1) {
			assert!(second_shard.contains(&m));
		}
	});
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
			ETHEREUM,
			pubkey_from_bytes(A),
			get_peer_id(A),
			90_000 * ANLOG,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(b.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(B),
			get_peer_id(B),
			90_000 * ANLOG,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(c.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(C),
			get_peer_id(C),
			90_000 * ANLOG,
		));
		Shards::create_shard(ETHEREUM, shard, 1);
		<pallet_shards::ShardState<Runtime>>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed(a.clone()).into(),
			TaskDescriptorParams {
				network: ETHEREUM,
				function: Function::EvmCall {
					address: Default::default(),
					input: Default::default(),
					amount: 0,
					gas_limit: None,
				},
				start: 0,
				funds: 10_000, //TODO: why does this underflow or below min error
				shard_size: 3,
			}
		));
		assert_eq!(<TaskSigner<Runtime>>::get(task_id), Some(pubkey_from_bytes(C)));
		roll_to(10);
		assert_eq!(<TaskSigner<Runtime>>::get(task_id), Some(pubkey_from_bytes(C)));
		roll_to(11);
		assert_eq!(<TaskSigner<Runtime>>::get(task_id), Some(pubkey_from_bytes(A)));
		roll_to(21);
		assert_eq!(<TaskSigner<Runtime>>::get(task_id), Some(pubkey_from_bytes(B)));
		roll_to(31);
		assert_eq!(<TaskSigner<Runtime>>::get(task_id), Some(pubkey_from_bytes(C)));
	});
}

#[test]
fn register_unregister_kills_task() {
	let a: AccountId = A.into();
	let b: AccountId = B.into();
	let c: AccountId = C.into();
	let d: AccountId = D.into();
	let e: AccountId = E.into();
	let old_shard = [a.clone(), b.clone(), c.clone()].to_vec();
	new_test_ext().execute_with(|| {
		assert_ok!(Members::register_member(
			RawOrigin::Signed(a.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(A),
			get_peer_id(A),
			90_000 * ANLOG,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(b.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(B),
			get_peer_id(B),
			90_000 * ANLOG,
		));
		assert_ok!(Members::register_member(
			RawOrigin::Signed(c.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(C),
			get_peer_id(C),
			90_000 * ANLOG,
		));
		// verify shard 0 created for Network Ethereum
		assert_eq!(Shards::shard_network(0), Some(ETHEREUM));
		<pallet_shards::ShardState<Runtime>>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		// create task
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed(a.clone()).into(),
			TaskDescriptorParams {
				network: ETHEREUM,
				function: Function::EvmCall {
					address: Default::default(),
					input: Default::default(),
					amount: 0,
					gas_limit: None,
				},
				start: 0,
				funds: 10_000, //TODO: why does this underflow if set to correct
				shard_size: 3,
			}
		));
		// verify task assigned to shard 0
		assert_eq!(Tasks::task_shard(0).unwrap(), 0);
		// member unregisters
		assert_ok!(Members::unregister_member(RawOrigin::Signed(a.clone()).into(),));
		// task still assigned to shard 0
		assert_eq!(Tasks::task_shard(0).unwrap(), 0);
		// member unregisters
		assert_ok!(Members::unregister_member(RawOrigin::Signed(b.clone()).into(),));
		Elections::shard_offline(ETHEREUM, old_shard);
		<pallet_shards::ShardState<Runtime>>::insert(0, ShardStatus::Offline);
		Tasks::shard_offline(0, ETHEREUM);
		// task no longer assigned
		assert!(Tasks::task_shard(0).is_none());
		// task killed
		assert!(Tasks::tasks(0).is_none());
		// new member
		assert_ok!(Members::register_member(
			RawOrigin::Signed(d.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(D),
			get_peer_id(D),
			90_001 * ANLOG,
		));
		// new member
		assert_ok!(Members::register_member(
			RawOrigin::Signed(e.clone()).into(),
			ETHEREUM,
			pubkey_from_bytes(E),
			get_peer_id(E),
			90_002 * ANLOG,
		));
		// verify shard 1 created for Network Ethereum
		assert_eq!(Shards::shard_network(1), Some(ETHEREUM));
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

#[test]
fn min_shard_stake_greater_than_register_unregister_task_rewards() {
	new_test_ext().execute_with(|| {
		let shard_size = <pallet_elections::ShardSize<Runtime>>::get();
		let read_task_reward: u128 = <Runtime as pallet_tasks::Config>::BaseReadReward::get();
		let send_message_reward: u128 =
			<Runtime as pallet_tasks::Config>::BaseSendMessageReward::get();
		let total_read_task_rewards = read_task_reward.saturating_mul(shard_size.into());
		let total_send_message_task_rewards = send_message_reward.saturating_mul(shard_size.into());
		let total_rewards_per_write_task: u128 = total_read_task_rewards
			.saturating_add(<Runtime as pallet_tasks::Config>::BaseWriteReward::get())
			.saturating_add(total_send_message_task_rewards);
		// fees are for 2 write tasks: RegisterShard and UnRegisterShard
		let shard_registration_fees = total_rewards_per_write_task.saturating_mul(2);
		let stake_per_member: u128 = <Runtime as pallet_members::Config>::MinStake::get();
		let min_shard_stake = stake_per_member.saturating_mul(shard_size.into());
		assert!(min_shard_stake > shard_registration_fees);
	});
}
