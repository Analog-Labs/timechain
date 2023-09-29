use crate::{Call, Config, Pallet};
use codec::alloc::string::ToString;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use time_primitives::{
	Function, Network, TaskDescriptorParams, TaskError, TaskResult, TasksInterface,
};

benchmarks! {
	create_task {}: _(RawOrigin::Signed(whitelisted_caller()), TaskDescriptorParams {
		network: Network::Ethereum,
		function: Function::EvmViewCall {
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
		let _ = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: Network::Ethereum,
			function: Function::EvmViewCall {
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
			function: Function::EvmViewCall {
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

	submit_result {
		let _ = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: Network::Ethereum,
			function: Function::EvmViewCall {
				address: Default::default(),
				function_signature: Default::default(),
				input: Default::default(),
			},
			cycle: 1,
			start: 0,
			period: 1,
			hash: "".to_string(),
		});
		Pallet::<T>::shard_online(1, Network::Ethereum);
	}: _(RawOrigin::Signed(whitelisted_caller()), 0, 0, TaskResult {
		shard_id: 1,
		hash: [0; 32],
		signature: [0; 64],
	}) verify {}

	submit_error {
		let _ = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: Network::Ethereum,
			function: Function::EvmViewCall {
				address: Default::default(),
				function_signature: Default::default(),
				input: Default::default(),
			},
			cycle: 1,
			start: 0,
			period: 1,
			hash: "".to_string(),
		});
		Pallet::<T>::shard_online(1, Network::Ethereum);
	}: _(RawOrigin::Signed(whitelisted_caller()), 0, 0, TaskError {
		shard_id: 1,
		msg: "test".to_string(),
		signature: [0; 64],
	}) verify {}

	submit_hash {
		let _ = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: Network::Ethereum,
			function: Function::EvmCall {
				address: Default::default(),
				function_signature: Default::default(),
				input: Default::default(),
				amount: 0,
			},
			cycle: 1,
			start: 0,
			period: 0,
			hash: "".to_string(),
		});
		Pallet::<T>::shard_online(1, Network::Ethereum);
	}: _(RawOrigin::Signed(whitelisted_caller()), 1, 0, "mock_hash".into()) verify {}

	submit_signature {
		let _ = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: Network::Ethereum,
			function: Function::SendMessage {
				message: Default::default(),
				input: Default::default(),
			},
			cycle: 1,
			start: 0,
			period: 0,
			hash: "".to_string(),
		});
		Pallet::<T>::shard_online(1, Network::Ethereum);
	}: _(RawOrigin::Signed(whitelisted_caller()), 0, [0u8; 64]) verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
