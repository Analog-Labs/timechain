use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::PartialEqNoBound;
use scale_info::TypeInfo;
use sp_std::prelude::*;

pub type MethodArguments = Vec<Vec<u8>>;
pub type Frequency = u64;
pub type TaskId = u64;

#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct OnchainTasks{
    pub task_id: TaskId,
    pub frequency: Frequency,
}

#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub struct OnChainTaskMetadata{
    pub task: SupportedTasks,
    pub arguments: MethodArguments,
}

#[derive(Clone, Copy, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum SupportedChain {
    Cosmos,
	Ethereum,
    Polkadot,
    Timechain,
}

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum SupportedTasks {
    EthereumTasks,
    CosmosTasks,
}

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum EthereumTasks {
    SwapToken,
	FetchEvents,
    FetchBalance,
    FetchBlocks,
}

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum CosmosTasks {
    FetchBlocks,
}

impl Default for SupportedChain {
    fn default() -> Self{
        SupportedChain::Timechain
    }
}


