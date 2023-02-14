use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::PartialEqNoBound;
use scale_info::TypeInfo;
use sp_std::prelude::*;

pub type MethodArguments = Vec<Vec<u8>>;
pub type Frequency = u64;
pub type TaskId = u64;

#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub struct OnChainTaskMetadata {
	pub task: SupportedTasks,
	pub arguments: MethodArguments,
}

#[derive(
	Clone, Copy, Encode, Default, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound,
)]
pub enum SupportedChain {
	Cosmos,
	Ethereum,
	Polkadot,
	#[default]
	Timechain,
}

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Eq, PartialEqNoBound)]
pub enum SupportedTasks {
	EthereumTasks(EthereumTasks),
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
