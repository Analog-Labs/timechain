use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_benchmarking::BenchmarkError;
use frame_system::RawOrigin;
use scale_info::prelude::string::String;

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

	remove_network {
		let name = String::from("chain_name");
		let network = String::from("chain_network");

		let network_id = Pallet::<T>::insert_network(name, network).map_err(|_| BenchmarkError::Stop("Faled to insert network, stopping benchmark"))?;
	}: _(RawOrigin::Root, network_id)
	verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
