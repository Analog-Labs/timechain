use super::*;
use crate::Pallet;

use frame_system::RawOrigin;
use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::frame_system;
use scale_info::prelude::string::String;
use time_primitives::NetworkId;

//TODO: choose & enforce MAX in code
const MAX_LENGTH: u32 = 1000;
const ETHEREUM: NetworkId = 0;

benchmarks! {
	register_network {
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
	}: _(RawOrigin::Root, 42, name, network, [0; 32], 20)
	verify {}

	set_network_config {
	}: _(RawOrigin::Root, ETHEREUM, 100, 25, 10_000, 10) verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
