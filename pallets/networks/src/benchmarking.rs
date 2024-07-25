use super::*;
use crate::Pallet;

use scale_info::prelude::string::String;

use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::frame_system;

use frame_system::RawOrigin;

//TODO: choose & enforce MAX in code
const MAX_LENGTH: u32 = 1000;

benchmarks! {
	add_network {
		let a in 1..MAX_LENGTH;
		let b in 1..MAX_LENGTH;
		let mut name = String::new();
		let mut network = String::new();
		for _ in 0..a {
			name.push('a');
		}
		for _ in 0..b {
			network.push('b');
		}
	}: _(RawOrigin::Root, name, network)
	verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
