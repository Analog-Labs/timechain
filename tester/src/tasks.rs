use anyhow::Result;
use std::collections::{BTreeMap, HashMap};
use subxt_signer::sr25519::dev;
use tc_subxt::{Function, Network, SubxtClient, TaskCreated, TaskDescriptorParams, TaskStatus};

pub async fn watch_task(api: &SubxtClient, task_id: u64) -> bool {
	let task_state = api.get_task_state(task_id).await.unwrap();
	let task_cycle = api.get_task_cycle(task_id).await.unwrap();
	println!("task_state: {:?}, task_cycle: {:?}", task_state, task_cycle);
	matches!(task_state, TaskStatus::Completed | TaskStatus::Failed { .. })
}

pub async fn watch_batch(
	api: &SubxtClient,
	start_index: u64,
	total_length: u64,
	max_cycle: u64,
) -> bool {
	let mut state_map: HashMap<u64, TaskStatus> = HashMap::new();
	let mut state_cycle: HashMap<u64, u64> = HashMap::new();
	for task_id in start_index..start_index + total_length {
		let task_state = api.get_task_state(task_id).await.unwrap();
		let task_cycle = api.get_task_cycle(task_id).await.unwrap_or_default();
		state_map.insert(task_id, task_state);
		state_cycle.insert(task_id, task_cycle);
	}
	let cycle_vals = state_cycle.values().collect::<Vec<_>>();
	let mut counts = BTreeMap::new();
	for item in cycle_vals {
		*counts.entry(item).or_insert(0) += 1;
	}
	println!("Batch cycles (cycle: tasks): {:?}", counts);
	if counts.get(&max_cycle).unwrap_or(&0) == &total_length {
		return true;
	}
	false
}

pub async fn insert_sign_task(
	api: &SubxtClient,
	cycle: u64,
	start: u64,
	period: u64,
	network: Network,
	contract_address: Vec<u8>,
	payload: Vec<u8>,
) -> Result<u64> {
	let function = Function::SendMessage { contract_address, payload };

	let params = TaskDescriptorParams {
		network,
		function,
		cycle,
		start,
		period,
		hash: "".to_string(),
	};
	let payload = SubxtClient::create_task_payload(params);
	let events = api.sign_and_submit_watch(&payload).await?;
	let transfer_event = events.find_first::<TaskCreated>().unwrap();
	let TaskCreated(id) = transfer_event.ok_or(anyhow::anyhow!("Not able to fetch task event"))?;
	println!("Task registered: {:?}", id);
	Ok(id)
}

pub async fn insert_evm_task(
	api: &SubxtClient,
	address: String,
	cycle: u64,
	start: u64,
	period: u64,
	network: Network,
	is_payable: bool,
) -> Result<u64> {
	let function = if is_payable {
		Function::EvmCall {
			address,
			function_signature: "function vote_yes()".to_string(),
			input: Default::default(),
			amount: 0,
		}
	} else {
		Function::EvmViewCall {
			address,
			function_signature: "function get_votes_stats() external view returns (uint[] memory)"
				.to_string(),
			input: Default::default(),
		}
	};

	let params = TaskDescriptorParams {
		network,
		function,
		cycle,
		start,
		period,
		hash: "".to_string(),
	};

	let payload = SubxtClient::create_task_payload(params);
	let events = api.sign_and_submit_watch(&payload).await?;

	let transfer_event = events.find_first::<TaskCreated>().unwrap();

	let TaskCreated(id) = transfer_event.ok_or(anyhow::anyhow!("Not able to fetch task event"))?;

	println!("Task registered: {:?}", id);
	Ok(id)
}
