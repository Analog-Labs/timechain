use anyhow::Result;
use clap::Parser;
use futures::{Stream, StreamExt};
use gmp_grpc::{proto, Gmp, GmpServer};
use gmp_rust::Connector;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use time_primitives::{
	ConnectorParams, IChain, IConnector, IConnectorAdmin, NetworkId, TssPublicKey,
};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

type GmpResult<T> = Result<Response<T>, Status>;

pub struct ConnectorWrapper {
	connector: Connector,
}

impl ConnectorWrapper {
	pub async fn new(network: NetworkId, db: &Path) -> Result<Self> {
		let connector = Connector::new(ConnectorParams {
			network_id: network,
			blockchain: "rust".into(),
			network: network.to_string(),
			url: db.to_str().unwrap().to_string(),
			mnemonic: String::new(),
		})
		.await?;
		Ok(Self { connector })
	}

	fn connector<T>(&self, req: Request<T>) -> Result<(Connector, T), Status> {
		let Some(addr) = req.metadata().get("address") else {
			return Err(Status::unauthenticated("No address provided"));
		};
		let Ok(addr) = gmp_rust::parse_address(addr.to_str().unwrap()) else {
			return Err(Status::unauthenticated("No valid address provided"));
		};
		let connector = self.connector.with_address(addr);
		let msg = req.into_inner();
		Ok((connector, msg))
	}
}

#[tonic::async_trait]
impl Gmp for ConnectorWrapper {
	async fn faucet(
		&self,
		request: Request<proto::FaucetRequest>,
	) -> GmpResult<proto::FaucetResponse> {
		let (connector, _) = self.connector(request)?;
		connector.faucet().await.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::FaucetResponse {}))
	}

	async fn transfer(
		&self,
		request: Request<proto::TransferRequest>,
	) -> GmpResult<proto::TransferResponse> {
		let (connector, msg) = self.connector(request)?;
		connector
			.transfer(msg.address, msg.amount)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::TransferResponse {}))
	}

	async fn balance(
		&self,
		request: Request<proto::BalanceRequest>,
	) -> GmpResult<proto::BalanceResponse> {
		let (connector, msg) = self.connector(request)?;
		let balance = connector
			.balance(msg.address)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::BalanceResponse { balance }))
	}

	type BlockStreamStream =
		Pin<Box<dyn Stream<Item = Result<proto::BlockStreamResponse, Status>> + Send + 'static>>;

	async fn block_stream(
		&self,
		request: Request<proto::BlockStreamRequest>,
	) -> GmpResult<Self::BlockStreamStream> {
		let (connector, _) = self.connector(request)?;
		let stream = connector.block_stream().map(|block| Ok(proto::BlockStreamResponse { block }));
		Ok(Response::new(stream.boxed()))
	}

	async fn read_events(
		&self,
		request: Request<proto::ReadEventsRequest>,
	) -> GmpResult<proto::ReadEventsResponse> {
		let (connector, msg) = self.connector(request)?;
		let events = connector
			.read_events(msg.gateway, msg.start_block..msg.end_block)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::ReadEventsResponse { events }))
	}

	async fn submit_commands(
		&self,
		request: Request<proto::SubmitCommandsRequest>,
	) -> GmpResult<proto::SubmitCommandsResponse> {
		let (connector, msg) = self.connector(request)?;
		connector
			.submit_commands(msg.gateway, msg.batch, msg.msg, msg.signer, msg.sig)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::SubmitCommandsResponse {}))
	}

	async fn deploy_gateway(
		&self,
		request: Request<proto::DeployGatewayRequest>,
	) -> GmpResult<proto::DeployGatewayResponse> {
		let (connector, msg) = self.connector(request)?;
		let (address, block) = connector
			.deploy_gateway(&msg.proxy, &msg.gateway)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::DeployGatewayResponse { address, block }))
	}

	async fn redeploy_gateway(
		&self,
		request: Request<proto::RedeployGatewayRequest>,
	) -> GmpResult<proto::RedeployGatewayResponse> {
		let (connector, msg) = self.connector(request)?;
		connector
			.redeploy_gateway(msg.proxy, &msg.gateway)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::RedeployGatewayResponse {}))
	}

	async fn admin(
		&self,
		request: Request<proto::AdminRequest>,
	) -> GmpResult<proto::AdminResponse> {
		let (connector, msg) = self.connector(request)?;
		let address = connector
			.admin(msg.gateway)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::AdminResponse { address }))
	}

	async fn set_admin(
		&self,
		request: Request<proto::SetAdminRequest>,
	) -> GmpResult<proto::SetAdminResponse> {
		let (connector, msg) = self.connector(request)?;
		connector
			.set_admin(msg.gateway, msg.admin)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::SetAdminResponse {}))
	}

	async fn shards(
		&self,
		request: Request<proto::ShardsRequest>,
	) -> GmpResult<proto::ShardsResponse> {
		let (connector, msg) = self.connector(request)?;
		let shards = connector
			.shards(msg.gateway)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		let shards = unsafe {
			std::mem::transmute::<Vec<TssPublicKey>, Vec<serde_big_array::Array<u8, 33>>>(shards)
		};
		Ok(Response::new(proto::ShardsResponse { shards }))
	}

	async fn set_shards(
		&self,
		request: Request<proto::SetShardsRequest>,
	) -> GmpResult<proto::SetShardsResponse> {
		let (connector, msg) = self.connector(request)?;
		let shards: Vec<TssPublicKey> = unsafe { std::mem::transmute(msg.shards) };
		connector
			.set_shards(msg.gateway, &shards)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::SetShardsResponse {}))
	}

	async fn networks(
		&self,
		request: Request<proto::NetworksRequest>,
	) -> GmpResult<proto::NetworksResponse> {
		let (connector, msg) = self.connector(request)?;
		let networks = connector
			.networks(msg.gateway)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::NetworksResponse { networks }))
	}

	async fn set_network(
		&self,
		request: Request<proto::SetNetworkRequest>,
	) -> GmpResult<proto::SetNetworkResponse> {
		let (connector, msg) = self.connector(request)?;
		connector
			.set_network(msg.gateway, msg.network)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::SetNetworkResponse {}))
	}

	async fn deploy_test(
		&self,
		request: Request<proto::DeployTestRequest>,
	) -> GmpResult<proto::DeployTestResponse> {
		let (connector, msg) = self.connector(request)?;
		let (address, block) = connector
			.deploy_test(msg.gateway, &msg.tester)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::DeployTestResponse { address, block }))
	}

	async fn estimate_message_cost(
		&self,
		request: Request<proto::EstimateMessageCostRequest>,
	) -> GmpResult<proto::EstimateMessageCostResponse> {
		let (connector, msg) = self.connector(request)?;
		let cost = connector
			.estimate_message_cost(msg.gateway, msg.dest, msg.msg_size)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::EstimateMessageCostResponse { cost }))
	}

	async fn send_message(
		&self,
		request: Request<proto::SendMessageRequest>,
	) -> GmpResult<proto::SendMessageResponse> {
		let (connector, msg) = self.connector(request)?;
		connector
			.send_message(msg.contract, msg.msg)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::SendMessageResponse {}))
	}

	async fn recv_messages(
		&self,
		request: Request<proto::RecvMessagesRequest>,
	) -> GmpResult<proto::RecvMessagesResponse> {
		let (connector, msg) = self.connector(request)?;
		let messages = connector
			.recv_messages(msg.contract, msg.start_block..msg.end_block)
			.await
			.map_err(|err| Status::unknown(err.to_string()))?;
		Ok(Response::new(proto::RecvMessagesResponse { messages }))
	}
}

#[derive(Parser)]
struct Args {
	#[arg(long)]
	network_id: NetworkId,
	#[arg(long)]
	port: u16,
	#[arg(long)]
	db: PathBuf,
}

async fn shutdown_signal() {
	let ctrl_c = async {
		tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
	};

	#[cfg(unix)]
	let terminate = async {
		tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
			.expect("failed to install signal handler")
			.recv()
			.await;
	};

	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();

	tokio::select! {
		_ = ctrl_c => {},
		_ = terminate => {},
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let args = Args::parse();
	let server = ConnectorWrapper::new(args.network_id, &args.db).await?;
	let svc = GmpServer::new(server);
	Server::builder()
		.add_service(svc)
		.serve_with_shutdown(SocketAddr::new([0, 0, 0, 0].into(), args.port), shutdown_signal())
		.await?;
	Ok(())
}
