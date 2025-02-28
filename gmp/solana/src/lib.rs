use std::{ops::Range, pin::Pin, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use solana_client::rpc_client::RpcClient;

use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

// TODO solana functions use self.invoke() which uses tokio::block_in_place. fix and use in async context

use time_primitives::{
	Address, BatchId, ConnectorParams, Gateway, GatewayMessage, GmpEvent, GmpMessage, IChain,
	IConnector, IConnectorAdmin, IConnectorBuilder, MessageId, NetworkId, Route, TssPublicKey,
	TssSignature,
};

fn a_addr(address: Address) -> Pubkey {
	Pubkey::new_from_array(address)
}

fn t_addr(pubkey: Pubkey) -> Address {
	pubkey.to_bytes()
}

pub struct Connector {
	network_id: NetworkId,
	client: RpcClient,
	wallet: Arc<Keypair>,
}

impl Connector {
	pub async fn send_transaction(&self, instruction: Instruction) -> Result<()> {
		let recent_blockhash = self.client.get_latest_blockhash().unwrap();
		let transaction = Transaction::new_signed_with_payer(
			&[instruction],
			Some(&self.wallet.pubkey()),
			&[&self.wallet],
			recent_blockhash,
		);
		let hash = self.client.send_and_confirm_transaction(&transaction).unwrap();
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
		let client = RpcClient::new(params.url);
		let connector = Self {
			network_id: params.network_id,
			client,
			wallet: Arc::new(Keypair::new()),
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
		self.client.request_airdrop(&self.wallet.pubkey(), balance as u64)?;
		Ok(())
	}
	async fn transfer(&self, address: Address, amount: u128) -> Result<()> {
		let instruction =
			system_instruction::transfer(&self.wallet.pubkey(), &a_addr(address), amount as u64);
		self.send_transaction(instruction).await
	}

	async fn balance(&self, address: Address) -> Result<u128> {
		let balance = self.client.get_balance(&a_addr(address))?;
		Ok(balance as u128)
	}

	async fn finalized_block(&self) -> Result<u64> {
		let block = self.client.get_slot_with_commitment(CommitmentConfig::finalized())?;
		Ok(block)
	}

	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + 'static>> {
		todo!()
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
		todo!("Not supported")
	}
	async fn redeploy_gateway(
		&self,
		_additional_params: &[u8],
		_proxy: Address,
		_gateway: &[u8],
	) -> Result<()> {
		todo!("Not supported")
	}
	async fn admin(&self, _gateway: Address) -> Result<Address> {
		todo!("Not supported")
	}
	async fn set_admin(&self, _gateway: Address, _admin: Address) -> Result<()> {
		todo!("Not supported")
	}
	async fn shards(&self, _gateway: Address) -> Result<Vec<TssPublicKey>> {
		todo!("Not supported")
	}
	async fn set_shards(&self, _gateway: Address, _keys: &[TssPublicKey]) -> Result<()> {
		todo!("Not supported")
	}
	async fn routes(&self, _gateway: Address) -> Result<Vec<Route>> {
		todo!("Not supported")
	}
	async fn set_route(&self, _gateway: Address, _route: Route) -> Result<()> {
		todo!("Not supported")
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
		todo!("Not supported")
	}
	async fn recv_messages(
		&self,
		_contract: Address,
		_blocks: Range<u64>,
	) -> Result<Vec<GmpMessage>> {
		todo!("Not supported")
	}
	async fn transaction_base_fee(&self) -> Result<u128> {
		// reference: <https://solana.com/docs/core/fees#key-points>
		Ok(5000)
	}
	async fn block_gas_limit(&self) -> Result<u64> {
		// reference: <https://solana.com/docs/core/fees#compute-units-and-limits>
		Ok(1_400_000)
	}

	async fn withdraw_funds(
		&self,
		_gateway: Address,
		_amount: u128,
		_address: Address,
	) -> Result<()> {
		todo!()
	}
}

#[async_trait]
impl IConnector for Connector {
	async fn read_events(&self, _gateway: Gateway, _blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
		todo!()
	}
	async fn submit_commands(
		&self,
		_gateway: Gateway,
		_batch: BatchId,
		_msg: GatewayMessage,
		_signer: TssPublicKey,
		_sig: TssSignature,
	) -> Result<(), String> {
		todo!()
	}
}
