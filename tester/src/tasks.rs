use anyhow::Result;
use sha3::{Digest, Keccak256};
use tc_subxt::timechain_runtime::runtime_types::time_primitives::task::TaskStatus;
use tc_subxt::timechain_runtime::tasks::events::{GatewayRegistered, TaskCreated};
use tc_subxt::SubxtClient;
use time_primitives::{Function, NetworkId, TaskDescriptorParams, TaskId};

pub async fn watch_task(api: &SubxtClient, task_id: TaskId) -> bool {
	let task_state = api.get_task_state(task_id).await.unwrap();
	println!("task_state: {:?}", task_state);
	matches!(task_state, Some(TaskStatus::Completed) | Some(TaskStatus::Failed { .. }))
}

pub fn create_evm_call(address: String) -> Function {
	Function::EvmCall {
		address: get_eth_address_to_bytes(&address),
		input: get_evm_function_hash("vote_yes()"),
		amount: 0,
	}
}

pub fn create_evm_view_call(address: String) -> Function {
	Function::EvmViewCall {
		address: get_eth_address_to_bytes(&address),
		input: get_evm_function_hash("get_votes_stats()"),
	}
}

pub fn create_send_msg_call(
	address: String,
	function: &str,
	salt: [u8; 32],
	gas_limit: u64,
) -> Function {
	Function::SendMessage {
		address: get_eth_address_to_bytes(&address),
		payload: get_evm_function_hash(function),
		salt,
		gas_limit,
	}
}

pub async fn insert_task(
	api: &SubxtClient,
	start: u64,
	network: NetworkId,
	function: Function,
) -> Result<u64> {
	let params = TaskDescriptorParams {
		network,
		function,
		start,
		shard_size: 1,
		funds: 0,
	};
	let events = api.create_task(params).await?.wait_for_finalized_success().await?;
	let transfer_event = events.find_first::<TaskCreated>().unwrap();
	let TaskCreated(id) = transfer_event.ok_or(anyhow::anyhow!("Not able to fetch task event"))?;
	println!("Task registered: {:?}", id);
	Ok(id)
}

pub async fn register_gateway_address(
	api: &SubxtClient,
	shard_id: u64,
	address: &str,
) -> Result<()> {
	let address_bytes = get_eth_address_to_bytes(address);
	let events = api
		.insert_gateway(shard_id, address_bytes.into())
		.await?
		.wait_for_finalized_success()
		.await?;
	let gateway_event = events.find_first::<GatewayRegistered>().unwrap();
	println!("Gateway registered with event {:?}", gateway_event);
	Ok(())
}

fn get_evm_function_hash(arg: &str) -> Vec<u8> {
	let mut hasher = Keccak256::new();
	hasher.update(arg);
	let hash = hasher.finalize();
	hash[..4].into()
}

fn get_eth_address_to_bytes(address: &str) -> [u8; 20] {
	let mut eth_bytes = [0u8; 20];
	let trimmed_address = address.trim_start_matches("0x");
	hex::decode_to_slice(trimmed_address, &mut eth_bytes).unwrap();
	eth_bytes
}
