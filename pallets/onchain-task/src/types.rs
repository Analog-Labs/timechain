use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{ PartialEqNoBound};
use scale_info::TypeInfo;
use sp_std::prelude::*;


/// types that uniquely identify a chain
pub type ChainKey = Vec<u8>;
pub type Chain = Vec<u8>;
pub type ChainEndpoint = Vec<u8>;
pub type Exchange = Vec<u8>;
pub type ExchangeAddress = Vec<u8>;
pub type ExchangeEndpoint = Vec<u8>;
pub type Token = Vec<u8>;
pub type TokenAddress = Vec<u8>;
pub type TokenEndpoint = Vec<u8>;
pub type SwapToken = Vec<u8>;
pub type SwapTokenAddress = Vec<u8>;
pub type SwapTokenEndpoint = Vec<u8>;


#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum TesseractTask {
    AddChain,
	FetchEvents,
    AddTokens,
    SwapTokens,
}
