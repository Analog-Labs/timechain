use crate::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::{ShardCreated, TimeId};

const A: TimeId = TimeId::new([1u8; 32]);

#[test]
fn test_ocw() {
	new_test_ext().execute_with(|| {
		Ocw::shard_created(1, vec![A]);
	});
}
