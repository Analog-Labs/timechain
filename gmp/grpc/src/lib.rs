use anyhow::Result;
use std::ops::Range;
use std::sync::Arc;
use time_primitives::{
	Address32, BatchId, GatewayMessage, GmpEvent, GmpMessage, Hash, IChain, IConnect, IConnector,
	IConnectorAdmin, MessageId, NetworkId, Route, TssPublicKey, TssSignature,
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
pub struct Chain(gmp_rust::Chain);

impl Chain {
	pub fn new(network: NetworkId, mnemonic: &str) -> Result<Self> {
		Ok(Self(gmp_rust::Chain::new(network, mnemonic)))
	}
}

#[tonic::async_trait]
impl IConnect for Chain {
	fn chain(&self) -> &dyn IChain {
		&self.0
	}

	async fn connect(&self, url: String) -> Result<Arc<dyn IConnector>> {
		Ok(Arc::new(Connector::new(self.clone(), url).await?))
	}

	async fn connect_admin(&self, url: String) -> Result<Arc<dyn IConnectorAdmin>> {
		Ok(Arc::new(Connector::new(self.clone(), url).await?))
	}
}

#[derive(Clone)]
struct AddressInterceptor {
	address: MetadataValue<Ascii>,
}

impl AddressInterceptor {
	fn new(address: String) -> Self {
		Self {
			address: address.parse().unwrap(),
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
	chain: Chain,
	client: Arc<Mutex<GmpClientT>>,
}

impl Connector {
	/// Creates a new connector.
	async fn new(chain: Chain, url: String) -> Result<Self>
	where
		Self: Sized,
	{
		let channel = if url.starts_with("https") {
			let tls_config = ClientTlsConfig::new().with_native_roots();
			Channel::from_shared(url)?.tls_config(tls_config)?.connect_lazy()
		} else {
			Channel::from_shared(url)?.connect_lazy()
		};
		let address = chain.chain().format_address(chain.chain().address());
		let client = GmpClient::with_interceptor(channel, AddressInterceptor::new(address));
		Ok(Self {
			chain,
			client: Arc::new(Mutex::new(client)),
		})
	}
}

#[tonic::async_trait]
impl IConnector for Connector {
	fn chain(&self) -> &dyn IChain {
		self.chain.chain()
	}
	async fn finalized_block(&self) -> Result<u64> {
		let request = Request::new(proto::FinalizedBlockRequest {});
		let response = self.client.lock().await.finalized_block(request).await?;
		Ok(response.get_ref().finalized_block)
	}
	/// Reads gmp messages from the target chain.
	async fn read_events(&self, gateway: Address32, blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
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
		gateway: Address32,
		batch: BatchId,
		msg: GatewayMessage,
		gas_price: u128,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<(), String> {
		let request = Request::new(proto::SubmitCommandsRequest {
			gateway,
			batch,
			msg,
			gas_price,
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
	/// Get EIP1559 `max_fee_per_gas` estimate for a chain.
	async fn gas_price(&self) -> Result<u128> {
		let request = Request::new(proto::GasPriceRequest {});
		let response = self.client.lock().await.gas_price(request).await?.into_inner();
		Ok(response.fee)
	}
}

#[tonic::async_trait]
impl IConnectorAdmin for Connector {
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self, balance: u128) -> Result<()> {
		let request = Request::new(proto::FaucetRequest { balance });
		self.client.lock().await.faucet(request).await?;
		Ok(())
	}
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address32, amount: u128) -> Result<()> {
		let request = Request::new(proto::TransferRequest { address, amount });
		self.client.lock().await.transfer(request).await?;
		Ok(())
	}
	/// Queries the account balance.
	async fn balance(&self, address: Address32) -> Result<u128> {
		let request = Request::new(proto::BalanceRequest { address });
		let response = self.client.lock().await.balance(request).await?;
		Ok(response.get_ref().balance)
	}
	/// Deploys the proxy contract.
	async fn deploy_gateway(&self, proxy: &[u8], gateway: &[u8]) -> Result<(Address32, u64)> {
		let request = Request::new(proto::DeployGatewayRequest {
			proxy: proxy.to_vec(),
			gateway: gateway.to_vec(),
		});
		let response = self.client.lock().await.deploy_gateway(request).await?.into_inner();
		Ok((response.address, response.block))
	}
	/// Deploys the gateway contract.
	async fn redeploy_gateway(&self, proxy: Address32, gateway: &[u8]) -> Result<()> {
		let request = Request::new(proto::RedeployGatewayRequest {
			proxy,
			gateway: gateway.to_vec(),
		});
		self.client.lock().await.redeploy_gateway(request).await?;
		Ok(())
	}
	/// Returns the gateway admin.
	async fn admin(&self, gateway: Address32) -> Result<Address32> {
		let request = Request::new(proto::AdminRequest { gateway });
		let response = self.client.lock().await.admin(request).await?.into_inner();
		Ok(response.address)
	}
	/// Sets the gateway admin.
	async fn set_admin(&self, gateway: Address32, admin: Address32) -> Result<()> {
		let request = Request::new(proto::SetAdminRequest { gateway, admin });
		self.client.lock().await.set_admin(request).await?;
		Ok(())
	}
	/// Returns the registered shard keys.
	async fn shards(&self, gateway: Address32) -> Result<Vec<TssPublicKey>> {
		let request = Request::new(proto::ShardsRequest { gateway });
		let response = self.client.lock().await.shards(request).await?.into_inner();
		Ok(unsafe {
			std::mem::transmute::<Vec<serde_big_array::Array<u8, 33>>, Vec<TssPublicKey>>(
				response.shards,
			)
		})
	}
	/// Sets the registered shard keys. Overwrites any other keys.
	async fn set_shards(
		&self,
		gateway: Address32,
		register: &[(TssPublicKey, u16)],
		revoke: &[(TssPublicKey, u16)],
	) -> Result<()> {
		let register =
			register.iter().copied().map(|(k, s)| (serde_big_array::Array(k), s)).collect();
		let revoke = revoke.iter().copied().map(|(k, s)| (serde_big_array::Array(k), s)).collect();
		let request = Request::new(proto::SetShardsRequest { gateway, register, revoke });
		self.client.lock().await.set_shards(request).await?;
		Ok(())
	}
	/// Returns the gateway routing table.
	async fn routes(&self, gateway: Address32) -> Result<Vec<Route>> {
		let request = Request::new(proto::RoutesRequest { gateway });
		let response = self.client.lock().await.routes(request).await?.into_inner();
		Ok(response.routes)
	}
	/// Updates an entry in the gateway routing table.
	async fn set_route(&self, gateway: Address32, route: Route) -> Result<()> {
		let request = Request::new(proto::SetRouteRequest { gateway, route });
		self.client.lock().await.set_route(request).await?;
		Ok(())
	}
	/// Updates all the prices in the gateway.
	async fn set_prices(&self, gateway: Address32, prices: &[f64]) -> Result<()> {
		let request = Request::new(proto::SetPricesRequest {
			gateway,
			prices: prices.to_vec(),
		});
		self.client.lock().await.set_prices(request).await?;
		Ok(())
	}
	/// Deploys a test contract.
	async fn deploy_tester(&self, gateway: Address32, tester: &[u8]) -> Result<(Address32, u64)> {
		let request = Request::new(proto::DeployTesterRequest {
			gateway,
			tester: tester.to_vec(),
		});
		let response = self.client.lock().await.deploy_tester(request).await?.into_inner();
		Ok((response.address, response.block))
	}
	/// Estimates the message gas limit.
	async fn estimate_message_gas_limit(
		&self,
		contract: Address32,
		src_network: NetworkId,
		src: Address32,
		payload: Vec<u8>,
	) -> Result<u64> {
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
		gateway: Address32,
		dest_network: NetworkId,
		msg_size: u16,
		gas_limit: u64,
	) -> Result<u128> {
		let request = Request::new(proto::EstimateMessageCostRequest {
			gateway,
			dest_network,
			msg_size,
			gas_limit,
		});
		let response = self.client.lock().await.estimate_message_cost(request).await?.into_inner();
		Ok(response.cost)
	}
	/// Sends a message using the test contract and returns the message id.
	async fn send_message(
		&self,
		src: Address32,
		dest_network: NetworkId,
		dest: Address32,
		gas_limit: u64,
		msg_cost: u128,
		payload: Vec<u8>,
	) -> Result<MessageId> {
		let request = Request::new(proto::SendMessageRequest {
			src,
			dest_network,
			dest,
			gas_limit,
			msg_cost,
			payload,
		});
		let response = self.client.lock().await.send_message(request).await?.into_inner();
		Ok(response.message_id)
	}

	/// Receives messages from test contract.
	async fn recv_messages(
		&self,
		contract: Address32,
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

	/// Returns gas limit of latest block.
	async fn block_gas_limit(&self) -> Result<u64> {
		let request = Request::new(proto::BlockGasLimitRequest {});
		let response = self.client.lock().await.block_gas_limit(request).await?.into_inner();
		Ok(response.gas_limit)
	}

	/// Withdraw gateway funds.
	async fn withdraw_funds(
		&self,
		gateway: Address32,
		amount: u128,
		address: Address32,
	) -> Result<()> {
		let request = Request::new(proto::WithdrawFundsRequest { gateway, amount, address });
		self.client.lock().await.withdraw_funds(request).await?.into_inner();
		Ok(())
	}

	/// Debug a transaction.
	async fn debug_transaction(&self, tx: Hash) -> Result<String> {
		let request = Request::new(proto::DebugTransactionRequest { tx });
		let response = self.client.lock().await.debug_transaction(request).await?.into_inner();
		Ok(response.details)
	}
}
