use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::PartialEqNoBound;
use scale_info::TypeInfo;
use sp_std::prelude::*;


/// types that uniquely identify a chain
/// types for storing chain data
pub type ChainKey = Vec<u8>;
pub type ChainData = Vec<u8>;


#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum TesseractTask {
    AddChain,
	FetchEvents,
    AddTokens,
    SwapTokens,
}

impl Default for TesseractTask {
    fn default() -> Self{
        TesseractTask::AddChain
    }
}

