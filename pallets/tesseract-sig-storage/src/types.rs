use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{ PartialEqNoBound};
use scale_info::TypeInfo;
use sp_std::prelude::*;
/// type that uniquely identify a signature data
pub type SignatureKey = Vec<u8>;

/// The type representing a signature data
pub type SignatureData = Vec<u8>;

#[derive(Clone, Encode, Decode,PartialEq,TypeInfo, Debug)]
pub struct SignatureStorage {
	pub signature_key: SignatureKey,
	pub signature_data: SignatureData,
	pub network_id: Vec<u8>,
	pub block_height: u64,
	pub time_stamp: u64
}

impl SignatureStorage {
	pub fn new (
		signature_key: SignatureKey,
		signature_data: SignatureData,
		network_id: Vec<u8>,
		block_height: u64,
		time_stamp:u64
	)-> Self {
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
