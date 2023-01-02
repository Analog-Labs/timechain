use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::PartialEqNoBound;
use scale_info::TypeInfo;
use sp_std::prelude::*;
pub use time_primitives::SignatureData;

/// type that uniquely identify a signature data
// pub type SignatureKey=<T>::Hash;//= T::Hash;// u64;//Vec<u8>;

#[derive(Clone, Encode, Decode, PartialEq, TypeInfo, Debug)]
pub struct SignatureStorage<S, T> {
	pub signature_key: S,
	pub signature_data: SignatureData,
	pub network_id: Vec<u8>,
	pub block_height: u64,
	pub time_stamp: T,
}

impl<S, T> SignatureStorage<S, T> {
	pub fn new(
		signature_key: S,
		signature_data: SignatureData,
		network_id: Vec<u8>,
		block_height: u64,
		time_stamp: T,
	) -> Self {
		SignatureStorage {
			signature_key,
			signature_data,
			network_id,
			block_height,
			time_stamp,
		}
	}
}

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum TesseractRole {
	Collector,
	Aggregator,
}
