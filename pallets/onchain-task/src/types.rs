use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::PartialEqNoBound;
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// type that uniquely identify a chain data
pub type ChainData = Vec<u8>;

/// The type representing a methods
pub type Methods = Vec<u8>;
// Struct for holding Onchain Task information.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub struct OnchainTaskData<Hash>{
    pub id: Hash,
    pub chain_data: ChainData,
    pub methods: Methods,
}
    


#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum TesseractTask {
    AddChain,
	FetchEvents,
    AddTokens,
    RunTask,
}



impl Default for TesseractTask {
    fn default() -> Self{
        TesseractTask::AddChain
    }
}


