use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::PartialEqNoBound;
use scale_info::TypeInfo;
use sp_std::prelude::*;

use std::cmp::Ordering;

pub type MethodArguments = Vec<Vec<u8>>;
pub type Frequency = u64;
pub type TaskId = u64;

#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq)]
pub struct OnchainTask {
	pub task_id: TaskId,
	pub frequency: Frequency,
}

impl PartialOrd for OnchainTask {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for OnchainTask {
	fn cmp(&self, other: &Self) -> Ordering {
		self.task_id.cmp(&other.task_id)
	}
}

// the task is unique if the same task id.
impl PartialEq for OnchainTask {
	fn eq(&self, other: &Self) -> bool {
		self.task_id == other.task_id
	}
}

#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub struct OnChainTaskMetadata {
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

impl Default for SupportedChain {
	fn default() -> Self {
		SupportedChain::Timechain
	}
}
