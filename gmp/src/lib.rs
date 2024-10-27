use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time_primitives::{ConnectorParams, TssPublicKey, TssSignature, GatewayMessage, BatchId, IChain, Route, NetworkId, Address, Address2, IConnector, IConnectorAdmin, IConnectorBuilder, GmpMessage, Gateway, GmpEvent};
use std::{pin::Pin, ops::Range};
use futures::Stream;

pub mod prelude {
	pub use time_primitives::{IChain, IConnector, IConnectorAdmin, IConnectorBuilder};
	pub use futures::Stream;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Backend {
	Evm,
	Grpc,
	Rust,
}

impl std::str::FromStr for Backend {
	type Err = anyhow::Error;

	fn from_str(backend: &str) -> Result<Self> {
		Ok(match backend {
			"evm" => Self::Evm,
			"grpc" => Self::Grpc,
			"rust" => Self::Rust,
			_ => anyhow::bail!("unsupported backend"),
		})
	}
}

#[derive(Clone)]
pub enum TypedConnector {
	Evm(gmp_evm::Connector),
	Grpc(gmp_grpc::Connector),
	Rust(gmp_rust::Connector),
}

#[async_trait::async_trait]
impl IChain for TypedConnector {
	type Address = Address2<32>;

	/// Formats an address into a string.
	fn format_address(&self, address: Address) -> String {
		match self {
			Self::Evm(connector) => connector.format_address(address),
			Self::Grpc(connector) => connector.format_address(address),
			Self::Rust(connector) => connector.format_address(address),
		}
	}
	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address> {
		Ok(match self {
			Self::Evm(connector) => connector.parse_address(address)?,
			Self::Grpc(connector) => connector.parse_address(address)?,
			Self::Rust(connector) => connector.parse_address(address)?,
		})
	}
	/// Returns the currency decimals and symobl.
	fn currency(&self) -> (u32, &str) {
		match self {
			Self::Evm(connector) => connector.currency(),
			Self::Grpc(connector) => connector.currency(),
			Self::Rust(connector) => connector.currency(),
		}
	}
	/// Formats a balance into a string.
	fn format_balance(&self, balance: u128) -> String {
		match self {
			Self::Evm(connector) => connector.format_balance(balance),
			Self::Grpc(connector) => connector.format_balance(balance),
			Self::Rust(connector) => connector.format_balance(balance),
		}
	}
	/// Parses a balance from a string.
	fn parse_balance(&self, balance: &str) -> Result<u128> {
		Ok(match self {
			Self::Evm(connector) => connector.parse_balance(balance)?,
			Self::Grpc(connector) => connector.parse_balance(balance)?,
			Self::Rust(connector) => connector.parse_balance(balance)?,
		})
	}
	/// Network identifier.
	fn network_id(&self) -> NetworkId {
		match self {
			Self::Evm(connector) => connector.network_id(),
			Self::Grpc(connector) => connector.network_id(),
			Self::Rust(connector) => connector.network_id(),
		}
	}
	/// Human readable connector account identifier.
	fn address(&self) -> Address {
		match self {
			Self::Evm(connector) => connector.address(),
			Self::Grpc(connector) => connector.address(),
			Self::Rust(connector) => connector.address(),
		}
	}
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self) -> Result<()> {
		Ok(match self {
			Self::Evm(connector) => connector.faucet().await?,
			Self::Grpc(connector) => connector.faucet().await?,
			Self::Rust(connector) => connector.faucet().await?,
		})
	}
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address, amount: u128) -> Result<()> {
		Ok(match self {
			Self::Evm(connector) => connector.transfer(address, amount).await?,
			Self::Grpc(connector) => connector.transfer(address, amount).await?,
			Self::Rust(connector) => connector.transfer(address, amount).await?,
		})
	}
	/// Queries the account balance.
	async fn balance(&self, address: Address) -> Result<u128> {
		Ok(match self {
			Self::Evm(connector) => connector.balance(address).await?,
			Self::Grpc(connector) => connector.balance(address).await?,
			Self::Rust(connector) => connector.balance(address).await?,
		})
	}
	/// Stream of finalized block indexes.
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + 'static>> {
		match self {
			Self::Evm(connector) => connector.block_stream(),
			Self::Grpc(connector) => connector.block_stream(),
			Self::Rust(connector) => connector.block_stream(),
		}
	}
} 

#[async_trait::async_trait]
impl IConnector for TypedConnector {
	/// Reads gmp messages from the target chain.
	async fn read_events(&self, gateway: Gateway, blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
		Ok(match self {
			Self::Evm(connector) => connector.read_events(gateway, blocks).await?,
			Self::Grpc(connector) => connector.read_events(gateway, blocks).await?,
			Self::Rust(connector) => connector.read_events(gateway, blocks).await?,
		})
	}
	/// Submits a gmp message to the target chain.
	async fn submit_commands(
		&self,
		gateway: Gateway,
		batch: BatchId,
		msg: GatewayMessage,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<(), String> {
		Ok(match self {
			Self::Evm(connector) => connector.submit_commands(gateway, batch, msg, signer, sig).await?,
			Self::Grpc(connector) => connector.submit_commands(gateway, batch, msg, signer, sig).await?,
			Self::Rust(connector) => connector.submit_commands(gateway, batch, msg, signer, sig).await?,
		})
	}
}

#[async_trait::async_trait]
impl IConnectorAdmin for TypedConnector {
	/// Deploys the gateway contract.
	async fn deploy_gateway(&self, proxy: &[u8], gateway: &[u8]) -> Result<(Address, u64)> {
		Ok(match self {
			Self::Evm(connector) => connector.deploy_gateway(proxy, gateway).await?,
			Self::Grpc(connector) => connector.deploy_gateway(proxy, gateway).await?,
			Self::Rust(connector) => connector.deploy_gateway(proxy, gateway).await?,
		})
	}
	/// Redeploys the gateway contract.
	async fn redeploy_gateway(&self, proxy: Address, gateway: &[u8]) -> Result<()> {
		Ok(match self {
			Self::Evm(connector) => connector.redeploy_gateway(proxy, gateway).await?,
			Self::Grpc(connector) => connector.redeploy_gateway(proxy, gateway).await?,
			Self::Rust(connector) => connector.redeploy_gateway(proxy, gateway).await?,
		})
	}
	/// Checks if the gateway needs to be redeployed.
	async fn gateway_needs_redeployment(&self, proxy: Address, gateway: &[u8]) -> Result<bool> {
		Ok(match self {
			Self::Evm(connector) => connector.gateway_needs_redeployment(proxy, gateway).await?,
			Self::Grpc(connector) => connector.gateway_needs_redeployment(proxy, gateway).await?,
			Self::Rust(connector) => connector.gateway_needs_redeployment(proxy, gateway).await?,
		})
	}
	/// Returns the gateway admin.
	async fn admin(&self, gateway: Address) -> Result<Address> {
		Ok(match self {
			Self::Evm(connector) => connector.admin(gateway).await?,
			Self::Grpc(connector) => connector.admin(gateway).await?,
			Self::Rust(connector) => connector.admin(gateway).await?,
		})
	}
	/// Sets the gateway admin.
	async fn set_admin(&self, gateway: Address, admin: Address) -> Result<()> {
		Ok(match self {
			Self::Evm(connector) => connector.set_admin(gateway, admin).await?,
			Self::Grpc(connector) => connector.set_admin(gateway, admin).await?,
			Self::Rust(connector) => connector.set_admin(gateway, admin).await?,
		})
	}
	/// Returns the registered shard keys.
	async fn shards(&self, gateway: Address) -> Result<Vec<TssPublicKey>> {
		Ok(match self {
			Self::Evm(connector) => connector.shards(gateway).await?,
			Self::Grpc(connector) => connector.shards(gateway).await?,
			Self::Rust(connector) => connector.shards(gateway).await?,
		})
	}
	/// Sets the registered shard keys. Overwrites any other keys.
	async fn set_shards(&self, gateway: Address, keys: &[TssPublicKey]) -> Result<()> {
		Ok(match self {
			Self::Evm(connector) => connector.set_shards(gateway, keys).await?,
			Self::Grpc(connector) => connector.set_shards(gateway, keys).await?,
			Self::Rust(connector) => connector.set_shards(gateway, keys).await?,
		})
	}
	/// Returns the gateway routing table.
	async fn routes(&self, gateway: Address) -> Result<Vec<Route>> {
		Ok(match self {
			Self::Evm(connector) => connector.routes(gateway).await?,
			Self::Grpc(connector) => connector.routes(gateway).await?,
			Self::Rust(connector) => connector.routes(gateway).await?,
		})
	}
	/// Updates an entry in the gateway routing table.
	async fn set_route(&self, gateway: Address, route: Route) -> Result<()> {
		Ok(match self {
			Self::Evm(connector) => connector.set_route(gateway, route).await?,
			Self::Grpc(connector) => connector.set_route(gateway, route).await?,
			Self::Rust(connector) => connector.set_route(gateway, route).await?,
		})
	}
	/// Deploys a test contract.
	async fn deploy_test(&self, gateway: Address, tester: &[u8]) -> Result<(Address, u64)> {
		Ok(match self {
			Self::Evm(connector) => connector.deploy_test(gateway, tester).await?,
			Self::Grpc(connector) => connector.deploy_test(gateway, tester).await?,
			Self::Rust(connector) => connector.deploy_test(gateway, tester).await?,
		})
	}
	/// Estimates the message cost.
	async fn estimate_message_cost(
		&self,
		gateway: Address,
		dest: NetworkId,
		msg_size: usize,
	) -> Result<u128> {
		Ok(match self {
			Self::Evm(connector) => connector.estimate_message_cost(gateway, dest, msg_size).await?,
			Self::Grpc(connector) => connector.estimate_message_cost(gateway, dest, msg_size).await?,
			Self::Rust(connector) => connector.estimate_message_cost(gateway, dest, msg_size).await?,
		})
	}
	/// Sends a message using the test contract.
	async fn send_message(&self, contract: Address, msg: GmpMessage) -> Result<()> {
		Ok(match self {
			Self::Evm(connector) => connector.send_message(contract, msg).await?,
			Self::Grpc(connector) => connector.send_message(contract, msg).await?,
			Self::Rust(connector) => connector.send_message(contract, msg).await?,
		})
	}
	/// Receives messages from test contract.
	async fn recv_messages(&self, contract: Address, blocks: Range<u64>)
		-> Result<Vec<GmpMessage>> {
		Ok(match self {
			Self::Evm(connector) => connector.recv_messages(contract, blocks).await?,
			Self::Grpc(connector) => connector.recv_messages(contract, blocks).await?,
			Self::Rust(connector) => connector.recv_messages(contract, blocks).await?,
		})
	}
}

impl Backend {
	pub async fn connect(&self, params: &ConnectorParams) -> Result<Arc<TypedConnector>> {
		Ok(match self {
			Self::Evm => Arc::new(TypedConnector::Evm(gmp_evm::Connector::new(params.clone()).await?)),
			Self::Grpc => Arc::new(TypedConnector::Grpc(gmp_grpc::Connector::new(params.clone()).await?)),
			Self::Rust => Arc::new(TypedConnector::Rust(gmp_rust::Connector::new(params.clone()).await?)),
		})
	}

	pub async fn connect_admin(
		&self,
		params: &ConnectorParams,
	) -> Result<Arc<TypedConnector>> {
		Ok(match self {
			Self::Evm => Arc::new(TypedConnector::Evm(gmp_evm::Connector::new(params.clone()).await?)),
			Self::Grpc => Arc::new(TypedConnector::Grpc(gmp_grpc::Connector::new(params.clone()).await?)),
			Self::Rust => Arc::new(TypedConnector::Rust(gmp_rust::Connector::new(params.clone()).await?)),
		})
	}
}
