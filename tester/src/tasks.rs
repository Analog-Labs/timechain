use crate::polkadot;
use crate::polkadot::runtime_types::time_primitives::task::TaskStatus;
use subxt::{OnlineClient, PolkadotConfig};

pub async fn get_task_state(
	api: &OnlineClient<PolkadotConfig>,
	task_id: u64,
) -> Option<TaskStatus> {
	let storage_query = polkadot::storage().tasks().task_state(task_id);
	api.storage().at_latest().await.unwrap().fetch(&storage_query).await.unwrap()
}
