use super::*;

#[allow(unused)]
use crate::Pallet as OnChainTask;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::format;
use sp_std::borrow::ToOwned;
use crate::{types::*};

benchmarks! {

	add_chain {
		let tesseract: T::AccountId = whitelisted_caller();
	}: _(tesseract.clone(), TesseractTask::AddChain)
	verify {
		assert_eq!(TesseractTasks::<T>::get(tesseract), Some(TesseractTask::AddChain));
	}

	store_onchain_task {

		let s in 0 .. 100;

		let tesseract: T::AccountId = whitelisted_caller();

		TesseractTasks::<T>::insert(tesseract.clone(), TesseractTask::AddChain);

		let chain_key =
			format!("{}{}","chain_key_".to_owned(),s).as_bytes().to_owned();

		let chain_data =
			format!("{}{}","this_is_the_chain".to_owned(),s).as_bytes().to_owned();


	}: _(RawOrigin::Signed(tesseract), chain_key, chain_data)
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain_key), Some(chain_data));
	}

	remove_chain {
		let tesseract: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root, tesseract.clone())
	verify {
		assert_eq!(TesseractTasks::<T>::get(tesseract), None);
	}

	impl_benchmark_test_suite!(OnChainTask, crate::mock::new_test_ext(), crate::mock::Test);
}