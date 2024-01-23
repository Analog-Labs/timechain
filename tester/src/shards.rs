use tc_subxt::timechain_runtime::runtime_types::time_primitives::shard::{Network, ShardStatus};
use tc_subxt::SubxtClient;

pub(crate) async fn is_shard_online(api: &SubxtClient, shard_id: u64) -> bool {
	api.shard_state(shard_id).await.unwrap() == ShardStatus::Online
}

pub(crate) async fn get_shard_id(api: &SubxtClient, network: Network) -> u64 {
	let shard_ids = api.shard_id_counter().await.expect("No shard available yet");
	let mut shard_id = 0;
	for i in 0..shard_ids {
		let shard_network = api.shard_network(i).await.unwrap();
		if shard_network == network.clone() {
			shard_id = i;
			break;
		}
	}
	shard_id
}
