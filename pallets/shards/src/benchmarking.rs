use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use time_primitives::{AccountId, Network};

fn shard() -> [AccountId; 3] {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	[a, b, c]
}

benchmarks! {
	register_shard {
	}: _(RawOrigin::Root, Network::Ethereum, shard().to_vec(), 1)
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
