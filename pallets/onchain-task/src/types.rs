use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::PartialEqNoBound;
use scale_info::TypeInfo;
use sp_std::prelude::*;


// Struct for holding Onchain Task information.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub struct OnchainTaskData<Hash>{
    pub id: Hash,
    pub chain_data: Vec<u8>,
    pub methods:Vec<u8>,
}
    


#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum TesseractTask {
    AddChain,
	FetchEvents,
    AddTokens,
    OnChainTask,
}



impl Default for TesseractTask {
    fn default() -> Self{
        TesseractTask::AddChain
    }
}


