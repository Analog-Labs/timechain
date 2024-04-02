use crate::{Call, Config, Pallet};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use pallet_shards::{ShardCommitment, ShardState};
use sp_std::vec;
use time_primitives::{
	AccountId, ElectionsInterface, Function, Msg, NetworkId, PublicKey, ShardStatus,
	ShardsInterface, TaskDescriptorParams, TaskResult, TasksInterface,
};

const ETHEREUM: NetworkId = 0;

fn public_key(acc: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(acc))
}

// Generated via `tests::bench_sig_helper`
// Returns pub_key, Signature
fn mock_submit_sig() -> ([u8; 33], [u8; 64]) {
	(
		[
			2, 36, 79, 43, 160, 29, 26, 4, 168, 242, 35, 104, 66, 1, 179, 183, 189, 197, 92, 84, 2,
			101, 52, 245, 230, 250, 199, 131, 188, 204, 228, 70, 248,
		],
		[
			173, 155, 111, 15, 236, 255, 38, 211, 249, 26, 133, 73, 206, 114, 9, 79, 124, 68, 253,
			165, 148, 129, 18, 88, 192, 162, 222, 56, 119, 56, 244, 184, 40, 168, 133, 185, 161,
			13, 69, 29, 16, 233, 116, 44, 67, 56, 152, 73, 90, 169, 128, 178, 106, 195, 156, 92,
			13, 62, 34, 215, 115, 94, 51, 80,
		],
	)
}

// Generated via `tests::bench_result_helper`
// Returns pub_key, TaskResult
fn mock_result_ok() -> ([u8; 33], TaskResult) {
	(
		[
			2, 36, 79, 43, 160, 29, 26, 4, 168, 242, 35, 104, 66, 1, 179, 183, 189, 197, 92, 84, 2,
			101, 52, 245, 230, 250, 199, 131, 188, 204, 228, 70, 248,
		],
		TaskResult {
			shard_id: 0,
			payload: time_primitives::Payload::Hashed([
				11, 210, 118, 190, 192, 58, 251, 12, 81, 99, 159, 107, 191, 242, 96, 233, 203, 127,
				91, 0, 219, 14, 241, 19, 45, 124, 246, 145, 176, 169, 138, 11,
			]),
			signature: [
				6, 7, 9, 187, 47, 68, 0, 246, 107, 215, 169, 76, 121, 8, 85, 213, 42, 253, 100, 32,
				62, 87, 85, 101, 146, 126, 200, 74, 76, 101, 188, 229, 30, 17, 202, 255, 105, 25,
				145, 174, 219, 202, 54, 185, 97, 39, 171, 219, 81, 123, 73, 35, 124, 32, 124, 148,
				155, 133, 40, 73, 165, 196, 167, 130,
			],
		},
	)
}

benchmarks! {
	where_clause {  where T: pallet_shards::Config + pallet_members::Config }
	create_task {
		let b in 1..10000;
		let input = vec![0u8; b as usize];
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::EvmViewCall {
				address: Default::default(),
				input,
			},
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		};
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let caller = whitelisted_caller();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(100_000_000_000_000),
		);
	}: _(RawOrigin::Signed(whitelisted_caller()), descriptor) verify {}

	submit_result {
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::EvmViewCall {
				address: Default::default(),
				input: Default::default(),
			},
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		};
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let caller = whitelisted_caller();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(100_000_000_000_000),
		);
		Pallet::<T>::create_task(RawOrigin::Signed(caller.clone()).into(), descriptor)?;
		let (pub_key, result) = mock_result_ok();
		ShardCommitment::<T>::insert(0, vec![pub_key]);
	}: _(RawOrigin::Signed(caller), 0, result) verify {}

	submit_hash {
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			start: 0,
			function: Function::EvmCall {
				address: Default::default(),
				input: Default::default(),
				amount: 0,
				gas_limit: None,
			},
			funds: 100,
			shard_size: 3,
		};
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let caller: AccountId= [0u8; 32].into();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(100_000_000_000_000),
		);
		Pallet::<T>::create_task(RawOrigin::Signed(caller.clone()).into(), descriptor)?;
	}: _(RawOrigin::Signed(caller), 0, [0u8; 32]) verify {}

	submit_signature {
		let function = Function::SendMessage { msg: Msg::default() };
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			start: 0,
			function: function.clone(),
			funds: 100,
			shard_size: <T as Config>::Elections::default_shard_size(),
		};
		// Fund and register all shard members
		let mut i = 0u8;
		while u16::from(i) < <T as Config>::Elections::default_shard_size() {
			let member = [i; 32];
			let member_account: AccountId = member.clone().into();
			pallet_balances::Pallet::<T>::resolve_creating(
				&member_account,
				pallet_balances::Pallet::<T>::issue(<T as pallet_members::Config>::MinStake::get() * 100),
			);
			pallet_members::Pallet::<T>::register_member(
				RawOrigin::Signed(member_account).into(),
				ETHEREUM,
				public_key(member.clone()),
				member,
				<T as pallet_members::Config>::MinStake::get(),
			)?;
			i += 1;
		}
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let raw_caller = [0u8; 32];
		let caller: AccountId = raw_caller.clone().into();
		Pallet::<T>::create_task(RawOrigin::Signed(caller.clone()).into(), descriptor)?;
		Pallet::<T>::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0)?;
		let (pub_key, signature) = mock_submit_sig();
		ShardCommitment::<T>::insert(0, vec![pub_key]);
	}: _(RawOrigin::Signed(caller), 0, signature) verify {}

	register_gateway {
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
	}: _(RawOrigin::Root, 0, [0u8; 20], 20) verify {}

	set_read_task_reward {
	}: _(RawOrigin::Root, 0, 20) verify {}

	set_write_task_reward {
	}: _(RawOrigin::Root, 0, 20) verify {}

	set_send_message_task_reward {
	}: _(RawOrigin::Root, 0, 20) verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
