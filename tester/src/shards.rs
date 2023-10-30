use crate::polkadot;
use crate::polkadot::runtime_types::time_primitives::shard::{Network, ShardStatus};
use subxt::{OnlineClient, PolkadotConfig};

pub(crate) async fn is_shard_online(api: &OnlineClient<PolkadotConfig>, network: Network) -> bool {
	let storage_query = polkadot::storage().shards().shard_id_counter();
	let shards_id = api
		.storage()
		.at_latest()
		.await
		.unwrap()
		.fetch(&storage_query)
		.await
		.unwrap()
		.unwrap();

	let mut shard_state = None;
	for i in 0..shards_id {
		let storage_query = polkadot::storage().shards().shard_network(i);
		let shard_network =
			api.storage().at_latest().await.unwrap().fetch(&storage_query).await.unwrap();

		if shard_network != Some(network.clone()) {
			continue;
		};

		let storage_query = polkadot::storage().shards().shard_state(i);
		shard_state = api.storage().at_latest().await.unwrap().fetch(&storage_query).await.unwrap();
		break;
	}

	shard_state == Some(ShardStatus::Online)
}
