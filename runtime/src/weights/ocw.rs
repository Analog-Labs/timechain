#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use crate::{self as pallet_ocw};
use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_ocw::WeightInfo for WeightInfo<T> {
	fn submit_tss_public_key() -> Weight {
		Weight::from_parts(17_389_000, 0)
	}
    fn submit_task_result() -> Weight {
		Weight::from_parts(17_389_000, 0)
    }
}
