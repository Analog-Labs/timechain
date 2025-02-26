use anyhow::Result;
use futures::{Stream, StreamExt};
use std::ops::Range;
use std::pin::Pin;
use std::sync::Arc;
use time_primitives::{
	Address, BatchId, ConnectorParams, Gateway, GatewayMessage, GmpEvent, GmpMessage, IChain,
	IConnector, IConnectorAdmin, IConnectorBuilder, MessageId, NetworkId, Route, TssPublicKey,
	TssSignature,
};
use tokio::sync::Mutex;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::service::interceptor::{InterceptedService, Interceptor};
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::{Request, Status};

mod codec;
pub mod proto;

mod gmp {
	include!(concat!(env!("OUT_DIR"), "/gmp.Gmp.rs"));
}

use gmp::gmp_client::GmpClient;
pub use gmp::gmp_server::{Gmp, GmpServer};

#[derive(Clone)]
struct AddressInterceptor {
	address: MetadataValue<Ascii>,
}

impl AddressInterceptor {
	fn new(address: Address) -> Self {
		Self {
			address: gmp_rust::format_address(address).parse().unwrap(),
		}
	}
}

impl Interceptor for AddressInterceptor {
	fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
		request.metadata_mut().insert("address", self.address.clone());
		Ok(request)
	}
}

type GmpClientT = GmpClient<InterceptedService<Channel, AddressInterceptor>>;

#[derive(Clone)]
pub struct Connector {
	network: NetworkId,
	address: Address,
	client: Arc<Mutex<GmpClientT>>,
}

#[tonic::async_trait]
impl IConnectorBuilder for Connector {
	/// Creates a new connector.
	async fn new(params: ConnectorParams) -> Result<Self>
	where
		Self: Sized,
	{
		let address = gmp_rust::mnemonic_to_address(params.mnemonic);
		let channel = if params.url.starts_with("https") {
			let tls_config = ClientTlsConfig::new().with_native_roots();
			Channel::from_shared(params.url)?.tls_config(tls_config)?.connect().await?
		} else {
			Channel::from_shared(params.url)?.connect().await?
		};
		let client = GmpClient::with_interceptor(channel, AddressInterceptor::new(address));
		Ok(Self {
			network: params.network_id,
			address,
			client: Arc::new(Mutex::new(client)),
		})
	}
}

#[tonic::async_trait]
impl IChain for Connector {
	/// Formats an address into a string.
	fn format_address(&self, address: Address) -> String {
		gmp_rust::format_address(address)
	}
	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address> {
		gmp_rust::parse_address(address)
	}
	fn currency(&self) -> (u32, &str) {
		gmp_rust::currency()
	}
	/// Network identifier.
	fn network_id(&self) -> NetworkId {
		self.network
	}
	/// Human readable connector account identifier.
	fn address(&self) -> Address {
		self.address
	}
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self, balance: u128) -> Result<()> {
		let request = Request::new(proto::FaucetRequest { balance });
		self.client.lock().await.faucet(request).await?;
		Ok(())
	}
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address, amount: u128) -> Result<()> {
		let request = Request::new(proto::TransferRequest { address, amount });
		self.client.lock().await.transfer(request).await?;
		Ok(())
	}
	/// Queries the account balance.
	async fn balance(&self, address: Address) -> Result<u128> {
		let request = Request::new(proto::BalanceRequest { address });
		let response = self.client.lock().await.balance(request).await?;
		Ok(response.get_ref().balance)
	}
	async fn finalized_block(&self) -> Result<u64> {
		let request = Request::new(proto::FinalizedBlockRequest {});
		let response = self.client.lock().await.finalized_block(request).await?;
		Ok(response.get_ref().finalized_block)
	}
	/// Stream of finalized block indexes.
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send>> {
		let request = Request::new(proto::BlockStreamRequest {});
		futures::executor::block_on(async move {
			self.client
				.lock()
				.await
				.block_stream(request)
				.await
				.unwrap()
				.into_inner()
				.filter_map(|res| async { res.ok().map(|msg| msg.block) })
				.boxed()
		})
	}
}

#[tonic::async_trait]
impl IConnector for Connector {
	/// Reads gmp messages from the target chain.
	async fn read_events(&self, gateway: Gateway, blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
		let request = Request::new(proto::ReadEventsRequest {
			gateway,
			start_block: blocks.start,
			end_block: blocks.end,
		});
		let response = self.client.lock().await.read_events(request).await?;
		Ok(response.into_inner().events)
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
		let request = Request::new(proto::SubmitCommandsRequest {
			gateway,
			batch,
			msg,
			signer,
			sig,
		});
		self.client
			.lock()
			.await
			.submit_commands(request)
			.await
			.map_err(|err| err.message().to_string())?;
		Ok(())
	}
}

#[tonic::async_trait]
impl IConnectorAdmin for Connector {
	/// Deploys the gateway contract.
	async fn deploy_gateway(
		&self,
		_additional_params: &[u8],
		proxy: &[u8],
		gateway: &[u8],
	) -> Result<(Address, u64)> {
		let request = Request::new(proto::DeployGatewayRequest {
			proxy: proxy.to_vec(),
			gateway: gateway.to_vec(),
		});
		let response = self.client.lock().await.deploy_gateway(request).await?.into_inner();
		Ok((response.address, response.block))
	}
	/// Redeploys the gateway contract.
	async fn redeploy_gateway(
		&self,
		_additional_params: &[u8],
		proxy: Address,
		gateway: &[u8],
	) -> Result<()> {
		let request = Request::new(proto::RedeployGatewayRequest {
			proxy,
			gateway: gateway.to_vec(),
		});
		self.client.lock().await.redeploy_gateway(request).await?;
		Ok(())
	}
	/// Returns the gateway admin.
	async fn admin(&self, gateway: Address) -> Result<Address> {
		let request = Request::new(proto::AdminRequest { gateway });
		let response = self.client.lock().await.admin(request).await?.into_inner();
		Ok(response.address)
	}
	/// Sets the gateway admin.
	async fn set_admin(&self, gateway: Address, admin: Address) -> Result<()> {
		let request = Request::new(proto::SetAdminRequest { gateway, admin });
		self.client.lock().await.set_admin(request).await?;
		Ok(())
	}
	/// Returns the registered shard keys.
	async fn shards(&self, gateway: Address) -> Result<Vec<TssPublicKey>> {
		let request = Request::new(proto::ShardsRequest { gateway });
		let response = self.client.lock().await.shards(request).await?.into_inner();
		Ok(unsafe {
			std::mem::transmute::<Vec<serde_big_array::Array<u8, 33>>, Vec<TssPublicKey>>(
				response.shards,
			)
		})
	}
	/// Sets the registered shard keys. Overwrites any other keys.
	async fn set_shards(&self, gateway: Address, keys: &[TssPublicKey]) -> Result<()> {
		let shards = keys.iter().copied().map(serde_big_array::Array).collect();
		let request = Request::new(proto::SetShardsRequest { gateway, shards });
		self.client.lock().await.set_shards(request).await?;
		Ok(())
	}
	/// Returns the gateway routing table.
	async fn routes(&self, gateway: Address) -> Result<Vec<Route>> {
		let request = Request::new(proto::RoutesRequest { gateway });
		let response = self.client.lock().await.routes(request).await?.into_inner();
		Ok(response.routes)
	}
	/// Updates an entry in the gateway routing table.
	async fn set_route(&self, gateway: Address, route: Route) -> Result<()> {
		let request = Request::new(proto::SetRouteRequest { gateway, route });
		self.client.lock().await.set_route(request).await?;
		Ok(())
	}
	/// Deploys a test contract.
	async fn deploy_test(
		&self,
		_additional_params: &[u8],
		gateway: Address,
		tester: &[u8],
	) -> Result<(Address, u64)> {
		let request = Request::new(proto::DeployTestRequest {
			gateway,
			tester: tester.to_vec(),
		});
		let response = self.client.lock().await.deploy_test(request).await?.into_inner();
		Ok((response.address, response.block))
	}
	/// Estimates the message gas limit.
	async fn estimate_message_gas_limit(
		&self,
		contract: Address,
		src_network: NetworkId,
		src: Address,
		payload: Vec<u8>,
	) -> Result<u128> {
		let request = Request::new(proto::EstimateMessageGasLimitRequest {
			contract,
			src_network,
			src,
			payload,
		});
		let response =
			self.client.lock().await.estimate_message_gas_limit(request).await?.into_inner();
		Ok(response.gas_limit)
	}
	/// Estimates the message cost.
	async fn estimate_message_cost(
		&self,
		gateway: Address,
		dest_network: NetworkId,
		gas_limit: u128,
		payload: Vec<u8>,
	) -> Result<u128> {
		let request = Request::new(proto::EstimateMessageCostRequest {
			gateway,
			dest_network,
			gas_limit,
			payload,
		});
		let response = self.client.lock().await.estimate_message_cost(request).await?.into_inner();
		Ok(response.cost)
	}
	/// Sends a message using the test contract and returns the message id.
	async fn send_message(
		&self,
		src: Address,
		dest_network: NetworkId,
		dest: Address,
		gas_limit: u128,
		gas_cost: u128,
		payload: Vec<u8>,
	) -> Result<MessageId> {
		let request = Request::new(proto::SendMessageRequest {
			src,
			dest_network,
			dest,
			gas_limit,
			gas_cost,
			payload,
		});
		let response = self.client.lock().await.send_message(request).await?.into_inner();
		Ok(response.message_id)
	}

	async fn send_cctp_message(
		&self,
		_gateway: Address,
		_src: Address,
		_dest_network: NetworkId,
		_dest: Address,
		_gas_limit: u128,
		_gas_cost: u128,
	) -> Result<MessageId> {
		anyhow::bail!("Not supported")
	}

	/// Receives messages from test contract.
	async fn recv_messages(
		&self,
		contract: Address,
		blocks: Range<u64>,
	) -> Result<Vec<GmpMessage>> {
		let request = Request::new(proto::RecvMessagesRequest {
			contract,
			start_block: blocks.start,
			end_block: blocks.end,
		});
		let response = self.client.lock().await.recv_messages(request).await?.into_inner();
		Ok(response.messages)
	}
	/// Calculate transaction base fee for a chain.
	async fn transaction_base_fee(&self) -> Result<u128> {
		let request = Request::new(proto::TransactionBaseFeeRequest {});
		let response = self.client.lock().await.transaction_base_fee(request).await?.into_inner();
		Ok(response.base_fee)
	}

	/// Returns gas limit of latest block.
	async fn block_gas_limit(&self) -> Result<u64> {
		let request = Request::new(proto::BlockGasLimitRequest {});
		let response = self.client.lock().await.block_gas_limit(request).await?.into_inner();
		Ok(response.gas_limit)
	}

	/// Withdraw gateway funds.
	async fn withdraw_funds(&self, gateway: Address, amount: u128, address: Address) -> Result<()> {
		let request = Request::new(proto::WithdrawFundsRequest { gateway, amount, address });
		self.client.lock().await.withdraw_funds(request).await?.into_inner();
		Ok(())
	}
}
