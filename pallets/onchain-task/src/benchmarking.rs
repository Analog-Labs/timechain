use super::*;

#[allow(unused)]
use crate::Pallet as OnChainTask;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::format;
use sp_std::borrow::ToOwned;
use crate::{types::*};

benchmarks! {

	add_task {
		let tesseract: T::AccountId = whitelisted_caller();
	}: _(tesseract.clone(), TesseractTask::AddChain)
	verify {
		assert_eq!(TesseractTasks::<T>::get(tesseract), Some(TesseractTask::AddChain));
	}

	store_onchain_data {

		let s in 0 .. 100;

		let tesseract: T::AccountId = whitelisted_caller();

		TesseractTasks::<T>::insert(tesseract.clone(), TesseractTask::AddChain);

		let chain_key =
			format!("{}{}","chain_key_".to_owned(),s).as_bytes().to_owned();

		let chain =
			format!("{}{}","this_is_the_chain".to_owned(),s).as_bytes().to_owned();

		let chain_endpoint =
			format!("{}{}","this_is_the_chain_endpoint".to_owned(),s).as_bytes().to_owned();

		let exchange =
			format!("{}{}","this_is_the_exchange".to_owned(),s).as_bytes().to_owned();

		let exchange_address =
			format!("{}{}","this_is_the_exchange_address".to_owned(),s).as_bytes().to_owned();

		let exchange_endpoint =	
			format!("{}{}","this_is_the_exchange_endpoint".to_owned(),s).as_bytes().to_owned();

		let token =
			format!("{}{}","this_is_the_token".to_owned(),s).as_bytes().to_owned();

		let token_address =
			format!("{}{}","this_is_the_token_address".to_owned(),s).as_bytes().to_owned();

		let token_endpoint =
			format!("{}{}","this_is_the_token_endpoint".to_owned(),s).as_bytes().to_owned();

		let swap_token =
			format!("{}{}","this_is_the_swap_token".to_owned(),s).as_bytes().to_owned();

		let swap_token_address =
			format!("{}{}","this_is_the_swap_token_address".to_owned(),s).as_bytes().to_owned();

		let swap_token_endpoint =
			format!("{}{}","this_is_the_swap_token_endpoint".to_owned(),s).as_bytes().to_owned();
			
	}: _(RawOrigin::Signed(tesseract), chain_key, chain, chain_endpoint, exchange, exchange_address, exchange_endpoint, token, token_address, token_endpoint, swap_token, swap_token_address, swap_token_endpoint)
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain_key), Some(chain), Some(chain_endpoint), Some(exchange), Some(exchange_address), Some(exchange_endpoint), Some(token), Some(token_address), Some(token_endpoint), Some(swap_token), Some(swap_token_address), Some(swap_token_endpoint));
	}

	remove_task {
		let tesseract: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root, tesseract.clone())
	verify {
		assert_eq!(TesseractTasks::<T>::get(tesseract), None);
	}

	impl_benchmark_test_suite!(OnChainTask, crate::mock::new_test_ext(), crate::mock::Test);
}