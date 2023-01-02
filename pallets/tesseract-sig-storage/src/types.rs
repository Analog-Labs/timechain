use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::PartialEqNoBound;
use scale_info::TypeInfo;
use sp_std::prelude::*;
pub use time_primitives::SignatureData;

/// type that uniquely identify a signature data
// pub type SignatureKey=<T>::Hash;//= T::Hash;// u64;//Vec<u8>;

#[derive(Clone, Encode, Decode, PartialEq, TypeInfo, Debug)]
pub struct SignatureStorage<T> {
	pub signature_data: SignatureData,
	pub time_stamp: T,
}

impl<T> SignatureStorage<T> {
	pub fn new(
		signature_data: SignatureData,
		time_stamp: T,
	) -> Self {
		SignatureStorage { signature_data, time_stamp }
	}
}

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum TesseractRole {
	Collector,
	Aggregator,
}
