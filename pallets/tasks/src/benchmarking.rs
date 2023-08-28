use crate::{Call, Config, Pallet};
use codec::alloc::string::ToString;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use time_primitives::{Function, Network, TaskDescriptorParams};


benchmarks! {
	create_task {}: _(RawOrigin::Signed(whitelisted_caller()), TaskDescriptorParams {
		network: Network::Ethereum,
		function: Function::EVMViewWithoutAbi {
			address: Default::default(),
			function_signature: Default::default(),
			input: Default::default(),
		},
		cycle: 1,
		start: 0,
		period: 1,
		hash: "".to_string(),
	}) verify {}

	stop_task {
		let _t = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: Network::Ethereum,
			function: Function::EVMViewWithoutAbi {
				address: Default::default(),
				function_signature: Default::default(),
				input: Default::default(),
			},
			cycle: 1,
			start: 0,
			period: 1,
			hash: "".to_string(),
		});
	}: _(RawOrigin::Signed(whitelisted_caller()), 0)
	verify { }

	resume_task {
		let _ = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: Network::Ethereum,
			function: Function::EVMViewWithoutAbi {
				address: Default::default(),
				function_signature: Default::default(),
				input: Default::default(),
			},
			cycle: 1,
			start: 0,
			period: 1,
			hash: "".to_string(),
		});
		let _ = Pallet::<T>::stop_task(RawOrigin::Signed(whitelisted_caller()).into(), 0);
	}: _(RawOrigin::Signed(whitelisted_caller()), 0)
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
