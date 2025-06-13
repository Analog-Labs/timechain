use crate::worker::Tx;
use crate::{metadata, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use subxt::utils::H256;
use time_primitives::{Address32, BlockHash, ChainName, Network, NetworkConfig, NetworkId};

impl SubxtClient {
	pub async fn register_network(&self, network: Network) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::RegisterNetwork { network }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn set_network_config(
		&self,
		network: NetworkId,
		config: NetworkConfig,
	) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::SetNetworkConfig { network, config }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn networks(&self, block: BlockHash) -> Result<Vec<NetworkId>> {
		let block = H256(block.0);
		let mut networks = vec![];
		let storage = metadata::storage().networks().networks_iter();
		let mut iter = self.client.storage().at(block).iter(storage).await?;
		while let Some(Ok(kv)) = iter.next().await {
			networks.push(kv.value);
		}
		Ok(networks)
	}

	pub async fn network_name(
		&self,
		network: NetworkId,
		block: BlockHash,
	) -> Result<Option<ChainName>> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().networks_api().network_name(network);
		let data: Option<ChainName> =
			self.client.runtime_api().at(block).call(runtime_call).await?.map(|name| name.0);
		Ok(data)
	}

	pub async fn network_gateway(
		&self,
		network: NetworkId,
		block: BlockHash,
	) -> Result<Option<Address32>> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().networks_api().network_gateway(network);
		let data = self.client.runtime_api().at(block).call(runtime_call).await?;
		Ok(data)
	}

	pub async fn network_config(
		&self,
		network: NetworkId,
		block: BlockHash,
	) -> Result<NetworkConfig> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().networks_api().network_config(network);
		let data = self.client.runtime_api().at(block).call(runtime_call).await?.0;
		Ok(data)
	}

	pub async fn network_gas_price(&self, network: NetworkId, block: BlockHash) -> Result<u128> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().networks_api().network_gas_price(network);
		let data = self.client.runtime_api().at(block).call(runtime_call).await?;
		Ok(data)
	}
}
