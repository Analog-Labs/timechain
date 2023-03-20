
#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]


//use frame_support::{traits::Get, weights::Weight};
use frame_support::{traits::Get, weights::{constants::RocksDbWeight,Weight}};
use crate::WeightInfo;
use sp_std::marker::PhantomData;
pub struct SigWeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SigWeightInfo<T> {
	
    fn store_schedule() -> Weight {
        Weight::from_ref_time(39_000_000_u64)
            // Standard Error: 2_151
            .saturating_add(Weight::from_ref_time(68_175_u64).saturating_mul(100_u64))
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }

}


impl WeightInfo for () {

    fn store_schedule() -> Weight {
		Weight::from_ref_time(33_000_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
    
}