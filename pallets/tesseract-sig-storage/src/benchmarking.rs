use super::*;

use crate::types::*;
#[allow(unused)]
use crate::Pallet as TesseractSigStorage;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::format;
use sp_std::borrow::ToOwned;

benchmarks! {

	add_member {
		let tesseract: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root, tesseract.clone(), TesseractRole::Collector)
	verify {
		assert_eq!(TesseractMembers::<T>::get(tesseract), Some(TesseractRole::Collector));
	}

	store_signature {

		let s in 0 .. 100;

		let tesseract: T::AccountId = whitelisted_caller();

		TesseractMembers::<T>::insert(tesseract.clone(), TesseractRole::Collector);

		let signature_key =
			format!("{}{}","signature_key_".to_owned(),s).as_bytes().to_owned();

		let signature_data =
			format!("{}{}","this_is_the_signature_data_".to_owned(),s).as_bytes().to_owned();

	}: _(RawOrigin::Signed(tesseract), signature_key.clone(), signature_data.clone())
	verify {
		assert_eq!(SignatureStore::<T>::get(signature_key), Some(signature_data));
	}

	remove_member {
		let tesseract: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root, tesseract.clone())
	verify {
		assert_eq!(TesseractMembers::<T>::get(tesseract), None);
	}

	impl_benchmark_test_suite!(TesseractSigStorage, crate::mock::new_test_ext(), crate::mock::Test);
}
