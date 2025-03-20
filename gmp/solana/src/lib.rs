use std::{ops::Range, pin::Pin, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;

use solana_client::rpc_config::{RpcBlockSubscribeConfig, RpcBlockSubscribeFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use time_primitives::{
	Address, BatchId, ConnectorParams, Gateway, GatewayMessage, GmpEvent, GmpMessage, IChain,
	IConnector, IConnectorAdmin, IConnectorBuilder, MessageId, NetworkId, Route, TssPublicKey,
	TssSignature,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

fn a_addr(address: Address) -> Pubkey {
	Pubkey::new_from_array(address)
}

fn t_addr(pubkey: Pubkey) -> Address {
	pubkey.to_bytes()
}

pub struct Connector {
	network_id: NetworkId,
	client: RpcClient,
	pubsub_client: Arc<PubsubClient>,
	wallet: Arc<Keypair>,
}

impl Connector {
	pub async fn send_transaction(&self, instruction: Instruction) -> Result<()> {
		let recent_blockhash = self.client.get_latest_blockhash().await?;
		let transaction = Transaction::new_signed_with_payer(
			&[instruction],
			Some(&self.wallet.pubkey()),
			&[&self.wallet],
			recent_blockhash,
		);
		let hash = self.client.send_and_confirm_transaction(&transaction).await?;
		tracing::info!("tx send with hash: {}", hash);
		Ok(())
	}
}

#[async_trait]
impl IConnectorBuilder for Connector {
	async fn new(params: ConnectorParams) -> Result<Self>
	where
		Self: Sized,
	{
		let urls: Vec<_> = params.url.split(";").collect();
		if urls.len() != 2 {
			anyhow::bail!("Invalid url for solana");
		}
		let http_url = urls[0];
		let ws_url = urls[1];
		let client = RpcClient::new(http_url.to_string());
		let pubsub_client = PubsubClient::new(ws_url).await?;
		let connector = Self {
			network_id: params.network_id,
			client,
			wallet: Arc::new(Keypair::new()),
			pubsub_client: Arc::new(pubsub_client),
		};
		Ok(connector)
	}
}

#[async_trait]
impl IChain for Connector {
	fn format_address(&self, address: Address) -> String {
		a_addr(address).to_string()
	}
	fn parse_address(&self, address: &str) -> Result<Address> {
		let pubkey: Pubkey = address.parse()?;
		Ok(t_addr(pubkey))
	}
	fn currency(&self) -> (u32, &str) {
		(9, "SOL")
	}
	fn network_id(&self) -> NetworkId {
		self.network_id
	}
	fn address(&self) -> Address {
		t_addr(self.wallet.pubkey())
	}
	async fn faucet(&self, balance: u128) -> Result<()> {
		// TODO add faucet for local devnode only
		self.client.request_airdrop(&self.wallet.pubkey(), balance as u64).await?;
		Ok(())
	}
	async fn transfer(&self, address: Address, amount: u128) -> Result<()> {
		let instruction =
			system_instruction::transfer(&self.wallet.pubkey(), &a_addr(address), amount as u64);
		self.send_transaction(instruction).await
	}

	async fn balance(&self, address: Address) -> Result<u128> {
		let balance = self.client.get_balance(&a_addr(address)).await?;
		Ok(balance as u128)
	}

	async fn finalized_block(&self) -> Result<u64> {
		let block = self.client.get_slot_with_commitment(CommitmentConfig::finalized()).await?;
		Ok(block)
	}

	// TODO add retry logic
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + 'static>> {
		let filter = RpcBlockSubscribeFilter::All;
		let config = RpcBlockSubscribeConfig {
			commitment: Some(CommitmentConfig::finalized()),
			encoding: None,
			transaction_details: None,
			show_rewards: Some(false),
			max_supported_transaction_version: None,
		};

		let pubsub_client = self.pubsub_client.clone();
		let (tx, rx) = mpsc::unbounded_channel::<u64>();

		tokio::spawn(async move {
			let block_subscribe = pubsub_client.block_subscribe(filter, Some(config));
			let (mut subscription, _) = block_subscribe.await.expect("Block subscription failed");

			while let Some(response) = subscription.next().await {
				let slot = response.value.slot;
				if tx.send(slot).is_err() {
					break;
				}
			}
		});

		Box::pin(UnboundedReceiverStream::new(rx))
	}
}

#[async_trait]
impl IConnectorAdmin for Connector {
	async fn deploy_gateway(
		&self,
		_additional_params: &[u8],
		_proxy: &[u8],
		_gateway: &[u8],
	) -> Result<(Address, u64)> {
		todo!("Need gateway implementation")
	}
	async fn redeploy_gateway(
		&self,
		_additional_params: &[u8],
		_proxy: Address,
		_gateway: &[u8],
	) -> Result<()> {
		todo!("Need gateway implementation")
	}
	async fn admin(&self, _gateway: Address) -> Result<Address> {
		todo!("Need gateway implementation")
	}
	async fn set_admin(&self, _gateway: Address, _admin: Address) -> Result<()> {
		todo!("Need gateway implementation")
	}
	async fn shards(&self, _gateway: Address) -> Result<Vec<TssPublicKey>> {
		todo!("Need gateway implementation")
	}
	async fn set_shards(&self, _gateway: Address, _keys: &[TssPublicKey]) -> Result<()> {
		todo!("Need gateway implementation")
	}
	async fn routes(&self, _gateway: Address) -> Result<Vec<Route>> {
		todo!("Need gateway implementation")
	}
	async fn set_route(&self, _gateway: Address, _route: Route) -> Result<()> {
		todo!("Need gateway implementation")
	}
	async fn deploy_test(&self, _gateway: Address, _tester: &[u8]) -> Result<(Address, u64)> {
		todo!("Not supported")
	}
	async fn estimate_message_gas_limit(
		&self,
		_contract: Address,
		_src_network: NetworkId,
		_src: Address,
		_payload: Vec<u8>,
	) -> Result<u128> {
		todo!("Not supported")
	}
	async fn estimate_message_cost(
		&self,
		_gateway: Address,
		_dest_network: NetworkId,
		_gas_limit: u128,
		_payload: Vec<u8>,
	) -> Result<u128> {
		todo!()
	}
	async fn send_message(
		&self,
		_src: Address,
		_dest_network: NetworkId,
		_dest: Address,
		_gas_limit: u128,
		_gas_cost: u128,
		_payload: Vec<u8>,
	) -> Result<MessageId> {
		todo!("Need gateway implementation")
	}
	async fn recv_messages(
		&self,
		_contract: Address,
		_blocks: Range<u64>,
	) -> Result<Vec<GmpMessage>> {
		todo!("Need gateway implementation")
	}
	async fn max_fee_per_gas(&self) -> Result<u128> {
		// reference: <https://solana.com/docs/core/fees#key-points>
		// 5000 per signature is base fee of solana
		Ok(5000)
	}

	async fn block_gas_limit(&self) -> Result<u64> {
		// reference: <https://solana.com/docs/core/fees#compute-units-and-limits>
		// single instruction can use upto 200k units
		// single transaction (multiple instructions) can use upto 1.4m units
		Ok(1_400_000)
	}

	async fn withdraw_funds(
		&self,
		_gateway: Address,
		_amount: u128,
		_address: Address,
	) -> Result<()> {
		todo!("Need gateway implementation")
	}
}

#[async_trait]
impl IConnector for Connector {
	async fn read_events(
		&self,
		_gateway: Gateway,
		_blocks: Range<u64>,
		_cctp_info: Option<(Vec<Address>, String)>,
	) -> Result<Vec<GmpEvent>> {
		todo!("Need gateway implementation")
	}
	async fn submit_commands(
		&self,
		_gateway: Gateway,
		_batch: BatchId,
		_msg: GatewayMessage,
		_signer: TssPublicKey,
		_sig: TssSignature,
	) -> Result<(), String> {
		todo!("Need gateway implementation")
	}
}
