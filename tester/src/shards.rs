use tc_subxt::SubxtClient;
use tc_subxt::{Network, ShardStatus};

pub(crate) async fn is_shard_online(api: &SubxtClient, network: Network) -> bool {
	let shards_id = api.shard_id_counter().await.unwrap();
	let mut shard_state = None;
	for i in 0..shards_id {
		let shard_network = api.shard_network(i).await.unwrap();
		if shard_network != network.clone() {
			continue;
		};

		shard_state = Some(api.shard_state(i).await.unwrap());
		break;
	}

	shard_state == Some(ShardStatus::Online)
}
