use std::str::FromStr;
use std::{ops::Range, pin::Pin, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;

use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::{RpcBlockSubscribeConfig, RpcBlockSubscribeFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::signature::Signature;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::UiTransactionEncoding;
use time_primitives::{
	Address32, BatchId, ConnectorParams, GatewayMessage, GmpEvent, GmpMessage, IChain, IConnector,
	IConnectorAdmin, IConnectorBuilder, MessageId, NetworkId, Route, TssPublicKey, TssSignature,
};
use tokio::sync::{mpsc, Semaphore};
use tokio_stream::wrappers::UnboundedReceiverStream;

fn a_addr(address: Address32) -> Pubkey {
	Pubkey::new_from_array(address)
}

fn t_addr(pubkey: Pubkey) -> Address32 {
	pubkey.to_bytes()
}

pub struct Connector {
	network_id: NetworkId,
	client: Arc<RpcClient>,
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
			client: Arc::new(client),
			wallet: Arc::new(Keypair::new()),
			pubsub_client: Arc::new(pubsub_client),
		};
		Ok(connector)
	}
}

#[async_trait]
impl IChain for Connector {
	fn format_address(&self, address: Address32) -> String {
		a_addr(address).to_string()
	}
	fn parse_address(&self, address: &str) -> Result<Address32> {
		let pubkey: Pubkey = address.parse()?;
		Ok(t_addr(pubkey))
	}
	fn currency(&self) -> (u32, &str) {
		(9, "SOL")
	}
	fn network_id(&self) -> NetworkId {
		self.network_id
	}
	fn address(&self) -> Address32 {
		t_addr(self.wallet.pubkey())
	}
	async fn faucet(&self, balance: u128) -> Result<()> {
		// TODO add faucet for local devnode only
		self.client.request_airdrop(&self.wallet.pubkey(), balance as u64).await?;
		Ok(())
	}
	async fn transfer(&self, address: Address32, amount: u128) -> Result<()> {
		let instruction =
			system_instruction::transfer(&self.wallet.pubkey(), &a_addr(address), amount as u64);
		self.send_transaction(instruction).await
	}

	async fn balance(&self, address: Address32) -> Result<u128> {
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
	// dont need proxy since solana programs are upgradable
	async fn deploy_gateway(
		&self,
		_additional_params: &[u8],
		_proxy: &[u8],
		gateway: &[u8],
	) -> Result<(Address32, u64)> {
		let program_keypair = Keypair::new();
		let program_pubkey = program_keypair.pubkey();
		let lamports = self.client.get_minimum_balance_for_rent_exemption(gateway.len()).await?;

		let create_account_ix = system_instruction::create_account(
			&self.wallet.pubkey(),
			&program_pubkey,
			lamports,
			0,
			&solana_sdk::loader_v4::id(),
		);

		let resize_ix = solana_sdk::loader_v4::set_program_length(
			&program_pubkey,
			&self.wallet.pubkey(),
			gateway.len() as u32,
			&self.wallet.pubkey(),
		);

		let write_ix = solana_sdk::loader_v4::write(
			&program_pubkey,
			&self.wallet.pubkey(),
			0,
			gateway.to_vec(),
		);

		let deploy_ix = solana_sdk::loader_v4::deploy(&program_pubkey, &self.wallet.pubkey());

		let recent_blockhash = self.client.get_latest_blockhash().await?;

		let transaction = Transaction::new_signed_with_payer(
			&[create_account_ix, resize_ix, write_ix, deploy_ix],
			Some(&self.wallet.pubkey()),
			&[&self.wallet, &program_keypair],
			recent_blockhash,
		);

		let signature = self.client.send_and_confirm_transaction(&transaction).await?;

		tracing::info!("Deployed gateway at address: {:?}", signature);

		let slot = self.client.get_slot().await?;

		Ok((t_addr(program_pubkey), slot))
	}

	async fn redeploy_gateway(
		&self,
		_additional_params: &[u8],
		proxy: Address32,
		gateway: &[u8],
	) -> Result<()> {
		let pubkey = a_addr(proxy);
		let retract_ix = solana_sdk::loader_v4::retract(&pubkey, &self.wallet.pubkey());

		let resize_ix = solana_sdk::loader_v4::set_program_length(
			&pubkey,
			&self.wallet.pubkey(),
			gateway.len() as u32,
			&self.wallet.pubkey(),
		);

		let write_ix =
			solana_sdk::loader_v4::write(&pubkey, &self.wallet.pubkey(), 0, gateway.to_vec());

		let deploy_ix = solana_sdk::loader_v4::deploy(&pubkey, &self.wallet.pubkey());

		let recent_blockhash = self.client.get_latest_blockhash().await?;
		let transaction = Transaction::new_signed_with_payer(
			&[retract_ix, resize_ix, write_ix, deploy_ix],
			Some(&self.wallet.pubkey()),
			&[&self.wallet],
			recent_blockhash,
		);
		self.client.send_and_confirm_transaction(&transaction).await?;
		Ok(())
	}

	async fn admin(&self, gateway: Address32) -> Result<Address32> {
		let program_id = a_addr(gateway);
		let (state_pda, _bump) = Pubkey::find_program_address(&[b"gateway_state"], &program_id);

		let data = self.client.get_account_data(&state_pda).await?;
		let state = GatewayState::try_deserialize(&mut data.as_slice())?;
		Ok(t_addr(state.admin))
	}

	async fn set_admin(&self, _gateway: Address32, _admin: Address32) -> Result<()> {
		todo!("Need gateway implementation")
	}

	async fn shards(&self, _gateway: Address32) -> Result<Vec<TssPublicKey>> {
		todo!("Need gateway implementation")
	}

	async fn set_shards(&self, _gateway: Address32, _keys: &[TssPublicKey]) -> Result<()> {
		todo!("Need gateway implementation")
	}

	async fn routes(&self, _gateway: Address32) -> Result<Vec<Route>> {
		todo!("Need gateway implementation")
	}

	async fn set_route(&self, _gateway: Address32, _route: Route) -> Result<()> {
		todo!("Need gateway implementation")
	}

	async fn deploy_test(&self, _gateway: Address32, _tester: &[u8]) -> Result<(Address32, u64)> {
		todo!("Not supported")
	}

	async fn estimate_message_gas_limit(
		&self,
		_contract: Address32,
		_src_network: NetworkId,
		_src: Address32,
		_payload: Vec<u8>,
	) -> Result<u128> {
		// Not supported
		Ok(0)
	}

	async fn estimate_message_cost(
		&self,
		_gateway: Address32,
		_dest_network: NetworkId,
		_gas_limit: u128,
		_payload: Vec<u8>,
	) -> Result<u128> {
		let msg = Message::new(&[], None);
		let fee = self.client.get_fee_for_message(&msg).await?;
		Ok(fee as u128)
	}

	async fn send_message(
		&self,
		_src: Address32,
		_dest_network: NetworkId,
		_dest: Address32,
		_gas_limit: u128,
		_gas_cost: u128,
		_payload: Vec<u8>,
	) -> Result<MessageId> {
		todo!("Need gateway implementation")
	}

	async fn recv_messages(
		&self,
		_contract: Address32,
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
		_gateway: Address32,
		_amount: u128,
		_address: Address32,
	) -> Result<()> {
		todo!("Need gateway implementation")
	}
}

#[async_trait]
impl IConnector for Connector {
	async fn read_events(
		&self,
		gateway: Address32,
		blocks: Range<u64>,
		_cctp_info: Option<(Vec<Address32>, String)>,
	) -> Result<Vec<GmpEvent>> {
		// 1. Get signatures with slot-based pagination
		let program_id = a_addr(gateway);
		let mut all_signatures = Vec::new();
		let mut before = None;
		let commitment = self.client.commitment();

		loop {
			let config = GetConfirmedSignaturesForAddress2Config {
				before: before.clone(),
				until: None,
				limit: Some(500),
				commitment: Some(commitment),
			};

			let signatures = self
				.client
				.get_signatures_for_address_with_config(&program_id, config)
				.await?
				.into_iter()
				.filter(|sig| blocks.contains(&sig.slot))
				.collect::<Vec<_>>();

			if signatures.is_empty() {
				break;
			}

			all_signatures.extend(signatures);
			// TODO remove unwrap
			before = all_signatures
				.last()
				.map(|s| Signature::from_str(&s.signature.clone()).unwrap());

			if let Some(last_slot) = all_signatures.last().map(|s| s.slot) {
				if last_slot < blocks.start {
					break;
				}
			}
		}

		let semaphore = Arc::new(Semaphore::new(10));
		let mut handles = vec![];

		for sig_info in all_signatures {
			let client = self.client.clone();
			let permit = semaphore.clone().acquire_owned().await?;

			handles.push(tokio::spawn(async move {
				let _permit = permit;
				// TODO remove unwrap
				let signature: Signature = sig_info.signature.parse().unwrap();
				match client.get_transaction(&signature, UiTransactionEncoding::JsonParsed).await {
					Ok(tx) => Ok((tx, sig_info)),
					Err(e) => {
						tracing::error!("Failed to fetch tx {}: {:?}", sig_info.signature, e);
						Err(e)
					},
				}
			}));
		}

		let mut events = Vec::new();
		for handle in handles {
			match handle.await {
				Ok(Ok((tx, sig_info))) => {
					if let Some(meta) = tx.transaction.meta {
						if let OptionSerializer::Some(logs) = meta.log_messages {
							for log in logs {
								if let Some(_event) = parse_event_from_log(log) {
									// TODO fix sig
									let _sig: Signature = sig_info.signature.parse().unwrap();
									let event =
										GmpEvent::BatchExecuted { batch_id: 0, tx_hash: None };
									events.push(event)
								}
							}
						}
					}
				},
				Ok(Err(e)) => tracing::warn!("Transaction processing failed: {:?}", e),
				Err(join_err) => tracing::error!("Task failed: {:?}", join_err),
			}
		}
		Ok(events)
	}
	async fn submit_commands(
		&self,
		_gateway: Address32,
		_batch: BatchId,
		_msg: GatewayMessage,
		_signer: TssPublicKey,
		_sig: TssSignature,
	) -> Result<(), String> {
		todo!("Need gateway implementation")
	}
}

fn parse_event_from_log(_log: String) -> Option<()> {
	todo!()
}
