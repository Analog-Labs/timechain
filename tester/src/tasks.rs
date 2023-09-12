use crate::polkadot;
use crate::polkadot::runtime_types::time_primitives::shard::Network;
use crate::polkadot::runtime_types::time_primitives::task::{
	Function, TaskDescriptorParams, TaskStatus,
};
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;

pub async fn get_task_state(
	api: &OnlineClient<PolkadotConfig>,
	task_id: u64,
) -> Option<TaskStatus> {
	let storage_query = polkadot::storage().tasks().task_state(task_id);
	api.storage().at_latest().await.unwrap().fetch(&storage_query).await.unwrap()
}

pub async fn insert_evm_task(
	api: &OnlineClient<PolkadotConfig>,
	address: String,
	cycle: u64,
	start: u64,
	period: u64,
) {
	let tx = polkadot::tx().tasks().create_task(TaskDescriptorParams {
		network: Network::Ethereum,
		function: Function::EvmViewCall {
			address,
			function_signature: "0x3de7086ce750513ef79d14eacbd1282c4e4b0cea".to_string(),
			input: Default::default(),
		},
		cycle,
		start,
		period,
		hash: "".to_string(),
	});

	let from = dev::alice();

	let events = api.tx().sign_and_submit_then_watch_default(&tx, &from)
        .await.unwrap()
        .wait_for_finalized_success()
        .await.unwrap();

	let transfer_event = events.find_first::<polkadot::tasks::events::TaskCreated>().unwrap();
    if let Some(event) = transfer_event {
        println!("Task reigistered: {event:?}");
    }
}
