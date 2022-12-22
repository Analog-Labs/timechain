use super::*;

use crate::types::*;
#[allow(unused)]
use crate::Pallet as OnChainTask;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_std::prelude::*;

benchmarks! {
	store_task {

		let s in 0 .. 100;

		let who: T::AccountId = whitelisted_caller();
		let chain: SupportedChain = SupportedChain::Timechain;
		let task_metadata = OnChainTaskMetadata {
			task: SupportedTasks::EthereumTasks(EthereumTasks::SwapToken),
			arguments: vec![vec![s as u8]],
		};

		let frequency = 100;

	}: _(RawOrigin::Signed(who), chain.clone(), task_metadata.clone(), frequency)

	impl_benchmark_test_suite!(OnChainTask, crate::mock::new_test_ext(), crate::mock::Test);
}
