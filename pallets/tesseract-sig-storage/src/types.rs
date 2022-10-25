use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{ PartialEqNoBound};
use scale_info::TypeInfo;
use sp_std::prelude::*;
/// type that uniquely identify a signature data
pub type SignatureKey = Vec<u8>;

/// The type representing a signature data
pub type SignatureData = Vec<u8>;

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum TesseractRole {
	Collector,
	Aggregator,
}
