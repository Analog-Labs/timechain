use crate::mock::*;
use frame_system::RawOrigin;
use pallet_shards::ShardState;
use time_primitives::{
	Function, NetworkId, ShardStatus, ShardsInterface, TaskDescriptorParams, TasksInterface,
};

const ETHEREUM: NetworkId = 0;

fn create_shard() {
	Shards::create_shard(
		ETHEREUM,
		[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
		1,
	);
	ShardState::<Test>::insert(0, ShardStatus::Online);
	Tasks::shard_online(0, ETHEREUM);
}

fn create_task() {
	Tasks::create_task(
		RawOrigin::Signed([0; 32].into()).into(),
		TaskDescriptorParams {
			network: ETHEREUM,
			start: 0,
			function: Function::EvmViewCall {
				address: Default::default(),
				input: Default::default(),
			},
			funds: 100,
			shard_size: 3,
		},
	)
	.unwrap();
}

#[test]
fn test_blocktime() {
	let _ = env_logger::try_init();
	new_test_ext().execute_with(|| {
		for _ in 0..100 {
			create_shard();
		}
		for _ in 0..10000 {
			create_task();
		}
		roll(1);
	});
}
