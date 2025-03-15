use std::future::Future;

use subxt::{runtime_api::RuntimeApi, storage::Storage, utils::H256, Error, PolkadotConfig};
use time_primitives::BlockHash;

use crate::{OnlineClient, SubxtClient};

mod members;
mod networks;
mod shards;
mod tasks;

impl SubxtClient {
	// Storage at or latest
	pub fn st_at_or_latest(
		&self,
		maybe_block: Option<BlockHash>,
	) -> impl Future<Output = Result<Storage<PolkadotConfig, OnlineClient>, Error>> + Send + 'static
	{
		let client = self.client.clone();
		async move {
			match maybe_block {
				Some(block) => {
					let block = H256::from(block.0);
					Ok(client.storage().at(block))
				},
				None => client.storage().at_latest().await,
			}
		}
	}

	// Runtime at or latest
	pub fn rt_at_or_latest(
		&self,
		maybe_block: Option<BlockHash>,
	) -> impl Future<Output = Result<RuntimeApi<PolkadotConfig, OnlineClient>, Error>> + Send + 'static
	{
		let client = self.client.clone();
		async move {
			match maybe_block {
				Some(block) => {
					let block = H256::from(block.0);
					Ok(client.runtime_api().at(block))
				},
				None => client.runtime_api().at_latest().await,
			}
		}
	}
}
