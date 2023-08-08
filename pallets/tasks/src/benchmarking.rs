use crate::{Call, Config, Pallet};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use time_primitives::{Function, Network, ScheduleInput};

benchmarks! {
	create_task {}: _(RawOrigin::Signed(whitelisted_caller()), ScheduleInput {
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

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
