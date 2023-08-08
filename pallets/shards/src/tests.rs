use crate::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::{Network, OcwSubmitTssPublicKey, PeerId, PublicKey};

const A: PeerId = [1u8; 32];
const B: PeerId = [2u8; 32];
const C: PeerId = [3u8; 32];
const D: PeerId = [4u8; 32];
const E: PeerId = [5u8; 32];
const F: PeerId = [6u8; 32];

fn collector() -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw([42; 32]))
}

#[test]
fn test_register_shard() {
	let shards = [[A, B, C], [C, B, A], [D, E, F]];
	new_test_ext().execute_with(|| {
		for shard in &shards {
			assert_ok!(Shards::register_shard(
				RawOrigin::Root.into(),
				Network::Ethereum,
				shard.to_vec(),
				collector(),
			),);
		}
		for (shard_id, shard) in shards.iter().enumerate() {
			let members = Shards::get_shard_members(shard_id as _);
			assert_eq!(members.len(), shard.len());
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
