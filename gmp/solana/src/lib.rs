use std::rc::Rc;
use std::str::FromStr;
use std::{ops::Range, pin::Pin, sync::Arc};

use anchor_client::anchor_lang::AnchorDeserialize;
use anchor_client::solana_sdk::signer::SeedDerivable;
use anchor_client::{Client as AnchorClient, Cluster};
use anyhow::Result;
use async_trait::async_trait;
use futures::{Stream, StreamExt};

use anchor_client::solana_client::nonblocking::pubsub_client::PubsubClient;
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use anchor_client::solana_client::rpc_config::{RpcBlockSubscribeConfig, RpcBlockSubscribeFilter};
use anchor_client::solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::message::Message;
use anchor_client::solana_sdk::signature::Signature;
use anchor_client::solana_sdk::signer::keypair::Keypair;
use anchor_client::solana_sdk::transaction::Transaction;
use anchor_client::solana_sdk::{self, system_instruction};
use anchor_client::solana_sdk::{pubkey::Pubkey, signer::Signer};

use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::UiTransactionEncoding;
use time_primitives::{
	Address32, BatchId, GatewayMessage, GmpEvent, GmpMessage, Hash, IChain, IConnect, IConnector,
	IConnectorAdmin, MessageId, NetworkId, Route, TssPublicKey, TssSignature,
};
use tokio::sync::Semaphore;
use types::{GatewayState, GmpPdaSeeds};

mod types;

pub fn a_addr(address: Address32) -> Pubkey {
	Pubkey::new_from_array(address)
}

pub fn t_addr(pubkey: Pubkey) -> Address32 {
	pubkey.to_bytes()
}

#[derive(Clone)]
pub struct Chain {
	network_id: NetworkId,
	wallet: Arc<Keypair>,
}

impl Chain {
	pub fn new(network_id: NetworkId, mnemonic: &str) -> Result<Self> {
		let keypair = Keypair::from_seed_phrase_and_passphrase(mnemonic, "")
			.map_err(|err| anyhow::anyhow!("{}", err.to_string()))?;
		Ok(Self {
			network_id,
			wallet: Arc::new(keypair),
		})
	}
}

#[async_trait]
impl IConnect for Chain {
	fn chain(&self) -> &dyn IChain {
		self
	}
	async fn connect(&self, url: String) -> Result<Arc<dyn IConnector>> {
		todo!()
	}
	async fn connect_admin(&self, url: String) -> Result<Arc<dyn IConnectorAdmin>> {
		todo!()
	}
}

impl IChain for Chain {
	fn network_id(&self) -> NetworkId {
		self.network_id
	}

	fn address(&self) -> Address32 {
		t_addr(self.wallet.pubkey())
	}

	fn format_address(&self, address: Address32) -> String {
		a_addr(address).to_string()
	}

	fn parse_address(&self, address: &str) -> Result<Address32> {
		let pubkey: Pubkey = address.parse()?;
		Ok(t_addr(pubkey))
	}
}

pub struct Connector {
	network_id: NetworkId,
	client: Arc<RpcClient>,
	pubsub_client: Arc<PubsubClient>,
	anchor_client: AnchorClient<Arc<Keypair>>,
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

impl Connector {
	async fn new(chain: Chain, url: String) -> Result<Self> {
		let ws_url = url.clone();
		let http_url = url.replace("ws", "http");
		let client = RpcClient::new(http_url.clone());
		let pubsub_client = PubsubClient::new(&ws_url).await?;
		let keypair = Keypair::new();
		let an_client = AnchorClient::new_with_options(
			Cluster::Custom(http_url, ws_url),
			Arc::new(keypair),
			CommitmentConfig {
				commitment: CommitmentLevel::Finalized,
			},
		);
		let connector = Self {
			network_id: chain.network_id,
			client: Arc::new(client),
			wallet: Arc::new(Keypair::new()),
			pubsub_client: Arc::new(pubsub_client),
			anchor_client: an_client,
		};
		Ok(connector)
	}
}

#[async_trait]
impl IConnectorAdmin for Connector {
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self, balance: u128) -> Result<()> {
		todo!()
	}
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address32, amount: u128) -> Result<()> {
		todo!()
	}

	/// Queries the account balance.
	async fn balance(&self, address: Address32) -> Result<u128> {
		todo!()
	}
	// dont need proxy since solana programs are upgradable
	async fn deploy_gateway(&self, _proxy: &[u8], gateway: &[u8]) -> Result<(Address32, u64)> {
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
	async fn redeploy_gateway(&self, proxy: Address32, gateway: &[u8]) -> Result<()> {
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
		let (state_pda, _bump) =
			Pubkey::find_program_address(&[&GmpPdaSeeds::State.to_seed()], &program_id);

		let data = self.client.get_account_data(&state_pda).await?;
		let state = GatewayState::deserialize(&mut data.as_slice())?;
		Ok(t_addr(state.admin))
	}
	async fn set_admin(&self, gateway: Address32, admin: Address32) -> Result<()> {
		let program = self.anchor_client.program(a_addr(gateway))?;
		let instruction = gmp_solana_contract::instruction::SetAdmin { new_admin: a_addr(admin) };
		let result = program.request().args(instruction);
		Ok(())
	}

	async fn shards(&self, gateway: Address32) -> Result<Vec<TssPublicKey>> {
		let program_id = a_addr(gateway);
		let (state_pda, _bump) =
			Pubkey::find_program_address(&[&GmpPdaSeeds::State.to_seed()], &program_id);

		let data = self.client.get_account_data(&state_pda).await?;
		let state = GatewayState::deserialize(&mut data.as_slice())?;
		let shards = state.shards.iter().map(|item| item.shard.clone().into()).collect();
		Ok(shards)
	}

	async fn set_shards(
		&self,
		gateway: Address32,
		register: &[(TssPublicKey, u16)],
		revoke: &[(TssPublicKey, u16)],
	) -> Result<()> {
		todo!("Need gateway implementation")
	}

	async fn routes(&self, gateway: Address32) -> Result<Vec<Route>> {
		let program_id = a_addr(gateway);
		let (state_pda, _bump) =
			Pubkey::find_program_address(&[&GmpPdaSeeds::State.to_seed()], &program_id);

		let data = self.client.get_account_data(&state_pda).await?;
		let state = GatewayState::deserialize(&mut data.as_slice())?;
		let routes = state.routes.iter().map(|item| item.clone().into()).collect();
		Ok(routes)
	}

	async fn set_route(&self, _gateway: Address32, _route: Route) -> Result<()> {
		todo!("Need gateway implementation")
	}

	/// Updates the prices of all routes.
	async fn set_prices(&self, gateway: Address32, prices: &[f64]) -> Result<()> {
		todo!()
	}

	/// Deploys test contract
	async fn deploy_tester(&self, gateway: Address32, tester: &[u8]) -> Result<(Address32, u64)> {
		todo!()
	}

	async fn estimate_message_gas_limit(
		&self,
		_contract: Address32,
		_src_network: NetworkId,
		_src: Address32,
		_payload: Vec<u8>,
	) -> Result<u64> {
		// Not supported
		Ok(0)
	}

	/// Estimates message cost
	async fn estimate_message_cost(
		&self,
		gateway: Address32,
		dest_network: NetworkId,
		msg_size: u16,
		gas_limit: u64,
	) -> Result<u128> {
		todo!()
	}

	async fn send_message(
		&self,
		_src: Address32,
		_dest_network: NetworkId,
		_dest: Address32,
		_gas_limit: u64,
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

	async fn debug_transaction(&self, _hash: Hash) -> Result<String> {
		todo!("Not available")
	}
}

#[async_trait]
impl IConnector for Connector {
	fn chain(&self) -> &dyn IChain {
		todo!()
	}

	/// Queries the latest finalized block.
	async fn finalized_block(&self) -> Result<u64> {
		todo!()
	}

	async fn read_events(&self, gateway: Address32, blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
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
		gateway: Address32,
		batch: BatchId,
		msg: GatewayMessage,
		gas_price: u128,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<(), String> {
		let gateway = a_addr(gateway);
		Ok(())
	}

	async fn gas_price(&self) -> Result<u128> {
		todo!()
	}
}

fn parse_event_from_log(_log: String) -> Option<()> {
	todo!()
}
