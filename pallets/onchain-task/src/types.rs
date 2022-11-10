use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::PartialEqNoBound;
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// type that uniquely identify a chain data
pub type ChainId = Vec<u8>;

/// type that uniquely identify a chain data
pub type ChainData = Vec<u8>;

/// The type representing a methods
pub type MethodName = Vec<u8>;
pub type MethodArguments = Vec<u8>;
// Struct for holding Onchain Task information.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub struct OnchainTaskData{
    pub chain_id: ChainId,
    pub chain_data: ChainData,
    pub methods: TaskMethod,
    pub task: ChainTask,
}

// Struct for holding Onchain Task information.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub struct TaskMethod{
    pub name: MethodName,
    pub arguments: MethodArguments,
}
    


#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum SupportedChain {
    Cosmos,
	Ethereum,
    Polkadot,
    Timechain,
}

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum ChainTask {
    SwapToken,
	FetchEvents,
    FetchBalance,
    FetchBlocks,
}



impl Default for SupportedChain {
    fn default() -> Self{
        SupportedChain::Timechain
    }
}


