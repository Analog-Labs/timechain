
use sp_api;
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
	}
}
