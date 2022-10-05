use super::*;

#[allow(unused)]
use crate::Pallet as TesseractSigStorage;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::format;

benchmarks! {
	store_signature {
		let s in 0 .. 100;

		let caller: T::AccountId = whitelisted_caller();
        let signature_key = 
            format!("{}{}","signature_key_".to_owned(),s).as_bytes().to_owned();

        let signature_data = 
            format!("{}{}","this_is_the_signature_data_".to_owned(),s).as_bytes().to_owned();
	}: _(RawOrigin::Signed(caller), signature_key.clone(), signature_data.clone())
	verify {
		assert_eq!(SignatureStore::<T>::get(signature_key), Some(signature_data));
	}

	impl_benchmark_test_suite!(TesseractSigStorage, crate::mock::new_test_ext(), crate::mock::Test);
}
