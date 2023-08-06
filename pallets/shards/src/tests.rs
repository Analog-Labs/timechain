use crate::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::{Network, ShardInterface, TimeId};

const A: TimeId = TimeId::new([1u8; 32]);
const B: TimeId = TimeId::new([2u8; 32]);
const C: TimeId = TimeId::new([3u8; 32]);
const D: TimeId = TimeId::new([4u8; 32]);
const E: TimeId = TimeId::new([5u8; 32]);
const F: TimeId = TimeId::new([6u8; 32]);

#[test]
fn test_register_shard() {
	let shards = [[A, B, C], [C, B, A], [D, E, F]];
	new_test_ext().execute_with(|| {
		for shard in &shards {
			assert_ok!(Shards::register_shard(
				RawOrigin::Root.into(),
				shard.to_vec(),
				Network::Ethereum,
			),);
		}
		for (shard_id, shard) in shards.iter().enumerate() {
			let members = Shards::get_shard_members(shard_id as _);
			assert_eq!(members.len(), shard.len());
			let collector = Shards::collector(shard_id as _);
			assert_eq!(collector, Some(shard[0].clone()));
		}
		for member in [A, B, C] {
			let shards = Shards::get_shards(member);
			assert_eq!(shards.len(), 2);
		}
		for (shard_id, _) in shards.iter().enumerate() {
			assert_ok!(Shards::submit_tss_public_key(shard_id as _, [0; 33]));
		}
	});
}
