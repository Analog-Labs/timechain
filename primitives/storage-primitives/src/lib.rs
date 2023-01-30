#![cfg_attr(not(feature = "std"), no_std)]

use sp_api;
use sp_std::vec::Vec;
use onchain_task_pallet::types::{OnchainTask, OnChainTaskMetadata};
pub type Frequency = u64;
pub type TaskId = u64;

sp_api::decl_runtime_apis! {
	pub trait GetStoreTask {
		#[allow(clippy::too_many_arguments)]
		fn task_store() -> Vec<Vec<OnchainTask>>;
	}

	pub trait GetTaskMetaData {
		#[allow(clippy::too_many_arguments)]
		fn task_metadata() ->  Vec<OnChainTaskMetadata>;
	}
}
