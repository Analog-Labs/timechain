use super::*;

use crate::types::*;
#[allow(unused)]
use crate::Pallet as TesseractSigStorage;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::format;
use sp_std::borrow::ToOwned;
use time_primitives::SignatureData;

// Check if last event generated by pallet is the one we're expecting
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

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
		let signature_data: SignatureData ="this_is_the_signature_data_1".as_bytes().to_owned();
		let network_id =
			format!("{}{}","network_id_".to_owned(),s).as_bytes().to_owned();
		let block_height = 1245;

	}: _(RawOrigin::Signed(tesseract), signature_data.clone(), network_id, block_height )

	remove_member {
		let tesseract: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root, tesseract.clone())
	verify {
		assert_eq!(TesseractMembers::<T>::get(tesseract), None);
	}
	
	submit_tss_group_key {
		let s in 1 .. 255;
		let key = [s as u8; 32];
	}: _(RawOrigin::None, s.into(), key)
	verify {
		assert_last_event::<T>(Event::<T>::NewTssGroupKey(s.into(), key).into());
	}

	impl_benchmark_test_suite!(TesseractSigStorage, crate::mock::new_test_ext(), crate::mock::Test);
}
