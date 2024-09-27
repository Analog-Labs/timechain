use super::*;
use crate::Pallet;

use scale_info::prelude::string::String;

use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::frame_system;

use frame_system::RawOrigin;

//TODO: choose & enforce MAX in code
const MAX_LENGTH: u32 = 1000;

benchmarks! {
	send_email {
		let a in 1..MAX_LENGTH;
		let b in 1..MAX_LENGTH;
		let mut to = String::new();
		let mut path = String::new();
		for _ in 0..a {
			to.push('a');
		}
		for _ in 0..b {
			path.push('b');
		}
	}: _(RawOrigin::Root, to, path)
	verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
