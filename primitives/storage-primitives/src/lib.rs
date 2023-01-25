#![cfg_attr(not(feature = "std"), no_std)]<<<<<<< HEAD
#![cfg_attr(not(feature = "std"), no_std)]

use onchain_task_pallet::types::{OnChainTaskMetadata, OnchainTask};
use sp_api;
use sp_std::vec::Vec;
pub type Frequency = u64;
pub type TaskId = u64;

sp_api::decl_runtime_apis! {
	pub trait GetStoreTask {
		#[allow(clippy::too_many_arguments)]
		fn task_store() -> Vec<OnchainTask>;
	}

	pub trait GetTaskMetaData {
		#[allow(clippy::too_many_arguments)]
		fn task_metadata() ->  Vec<OnChainTaskMetadata>;
=======

use sp_api;
use sp_std::vec::Vec;
pub type Frequency = u64;
pub type TaskId = u64;

pub struct OnchainTask {
	pub task_id: TaskId,
	pub frequency: Frequency,
}
sp_api::decl_runtime_apis! {

	pub trait GetStoreTask {
		#[allow(clippy::too_many_arguments)]
		fn task_store();
>>>>>>> add runtime api for task store
	}
}
