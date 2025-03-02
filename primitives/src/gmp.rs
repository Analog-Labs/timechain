use crate::{NetworkId, TssPublicKey};
use scale_codec::{Decode, Encode};
use scale_info::{prelude::vec::Vec, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

pub type Address = [u8; 32];
pub type Gateway = Address;
pub type MessageId = [u8; 32];
pub type Hash = [u8; 32];
pub type BatchId = u64;

const GMP_VERSION: &str = "Analog GMP v2";

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct GmpParams {
	pub network: NetworkId,
	pub gateway: Gateway,
}

impl GmpParams {
	pub fn new(network: NetworkId, gateway: Gateway) -> Self {
		Self { network, gateway }
	}

	pub fn hash(&self, payload: &[u8]) -> Vec<u8> {
		let mut data: Vec<u8> = Vec::new();
		data.extend_from_slice(GMP_VERSION.as_bytes());
		data.extend_from_slice(&self.network.to_be_bytes());
		data.extend_from_slice(&self.gateway);
		data.extend_from_slice(payload);
		data
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, Decode, Encode, TypeInfo, Eq, PartialEq, Ord, PartialOrd)]
pub struct GmpMessage {
	pub src_network: NetworkId,
	pub dest_network: NetworkId,
	pub src: Address,
	pub dest: Address,
	pub nonce: u64,
	pub gas_limit: u128,
	pub gas_cost: u128,
	pub bytes: Vec<u8>,
}

impl GmpMessage {
	const HEADER_LEN: usize = 224;

	pub fn encoded_len(&self) -> usize {
		Self::HEADER_LEN + self.bytes.len()
	}

	fn encode_header(&self) -> [u8; 224] {
		// we dont include gasCost here due to its dynamic nature
		let mut hdr = [0u8; 224];
		// Leaving initial 32 bytes with padded 0's
		hdr[32..64].copy_from_slice(&self.src);
		// Leaving 30 bytes with padded 0's
		hdr[94..96].copy_from_slice(&self.src_network.to_be_bytes());
		hdr[96..128].copy_from_slice(&self.dest);
		// Leaving 30 bytes with padded 0's
		hdr[158..160].copy_from_slice(&self.dest_network.to_be_bytes());
		// Leaving 16 bytes with padded 0's
		hdr[176..192].copy_from_slice(&self.gas_limit.to_be_bytes());
		// Leaving 16 bytes with padded 0's
		hdr[216..224].copy_from_slice(&self.nonce.to_be_bytes());
		hdr
	}

	pub fn encode_to(&self, buf: &mut Vec<u8>) {
		let msg_hash: [u8; 32] = Keccak256::digest(&self.bytes).into();
		buf.extend_from_slice(&self.encode_header());
		buf.extend_from_slice(&msg_hash);
	}

	pub fn message_id(&self) -> MessageId {
		let mut buf = Vec::new();
		self.encode_to(&mut buf);
		Keccak256::digest(buf).into()
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for GmpMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&hex::encode(self.message_id()))
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum GatewayOp {
	SendMessage(GmpMessage),
	RegisterShard(
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
	),
	UnregisterShard(
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
	),
}

impl GatewayOp {
	fn to_code(&self) -> u8 {
		match self {
			GatewayOp::SendMessage(_) => 1,
			GatewayOp::RegisterShard(_) => 2,
			GatewayOp::UnregisterShard(_) => 3,
		}
	}

	fn to_u256_code(&self) -> [u8; 32] {
		let mut bytes = [0u8; 32];
		bytes[31] = self.to_code();
		bytes
	}

	pub fn encoded_len(&self) -> usize {
		match self {
			Self::SendMessage(msg) => msg.encoded_len(),
			_ => 32,
		}
	}

	pub fn encode_to(&self, buf: &mut Vec<u8>) {
		match self {
			Self::SendMessage(msg) => {
				buf.extend_from_slice(&msg.message_id());
			},
			Self::RegisterShard(pubkey) => {
				let mut op_bytes = [0u8; 64];
				op_bytes[31..].copy_from_slice(pubkey);
				let op_hash: [u8; 32] = Keccak256::digest(op_bytes).into();
				buf.extend_from_slice(&op_hash);
			},
			Self::UnregisterShard(pubkey) => {
				let mut op_bytes = [0u8; 64];
				op_bytes[31..].copy_from_slice(pubkey);
				let op_hash: [u8; 32] = Keccak256::digest(op_bytes).into();
				buf.extend_from_slice(&op_hash);
			},
		}
	}

	pub fn gas(&self) -> u128 {
		match self {
			Self::SendMessage(msg) => msg.gas_cost,
			_ => 40_000,
		}
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for GatewayOp {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::SendMessage(msg) => {
				writeln!(f, "send_message {}", hex::encode(msg.message_id()))
			},
			Self::RegisterShard(key) => {
				writeln!(f, "register_shard {}", hex::encode(key))
			},
			Self::UnregisterShard(key) => {
				writeln!(f, "unregister_shard {}", hex::encode(key))
			},
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct GatewayMessage {
	pub ops: Vec<GatewayOp>,
}

impl GatewayMessage {
	pub fn new(ops: Vec<GatewayOp>) -> Self {
		Self { ops }
	}

	pub fn encode(&self, batch_id: BatchId) -> [u8; 32] {
		let mut op_root_hash: [u8; 32] = [0u8; 32];
		let mut buf = Vec::new();
		// include version in buffer
		buf.extend_from_slice(&[0u8; 32]);
		// include batch id
		// padding for batch_id, since batch_id is uint64 we do intial padding with 0 until we have 32 bytes
		buf.extend_from_slice(&[0u8; 24]);
		buf.extend_from_slice(&batch_id.to_be_bytes());
		for op in &self.ops {
			let mut op_hasher = Keccak256::new();
			let mut op_hash = Vec::new();
			op.encode_to(&mut op_hash);

			op_hasher.update(op_root_hash);
			op_hasher.update(op.to_u256_code());
			op_hasher.update(op_hash);
			op_root_hash = op_hasher.finalize().into();
		}
		buf.extend_from_slice(&op_root_hash);
		Keccak256::digest(&buf).into()
	}

	pub fn gas(&self) -> u128 {
		self.ops.iter().fold(0u128, |acc, op| acc.saturating_add(op.gas()))
	}
}

pub struct BatchBuilder {
	batch_gas_limit: u128,
	gas: u128,
	ops: Vec<GatewayOp>,
}

impl BatchBuilder {
	pub fn new(batch_gas_limit: u128) -> Self {
		Self {
			batch_gas_limit,
			gas: 0,
			ops: Default::default(),
		}
	}

	pub fn set_gas_limit(&mut self, batch_gas_limit: u128) {
		self.batch_gas_limit = batch_gas_limit;
	}

	pub fn take_batch(&mut self) -> Option<GatewayMessage> {
		if self.ops.is_empty() {
			return None;
		}
		self.gas = 0;
		let ops = core::mem::take(&mut self.ops);
		Some(GatewayMessage::new(ops))
	}

	pub fn push(&mut self, op: GatewayOp) -> Option<GatewayMessage> {
		let gas = op.gas();
		let batch = if self.gas + gas > self.batch_gas_limit { self.take_batch() } else { None };
		self.ops.push(op);
		batch
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, Eq, PartialEq, Ord, PartialOrd)]
pub enum GmpEvent {
	ShardRegistered(
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
	),
	ShardUnregistered(
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
	),
	MessageReceived(GmpMessage),
	MessageExecuted(MessageId),
	BatchExecuted {
		batch_id: BatchId,
		tx_hash: Option<Hash>,
	},
}

#[cfg(feature = "std")]
impl std::fmt::Display for GmpEvent {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::ShardRegistered(key) => {
				writeln!(f, "shard_registered {}", hex::encode(key))
			},
			Self::ShardUnregistered(key) => {
				writeln!(f, "shard_unregistered {}", hex::encode(key))
			},
			Self::MessageReceived(msg) => {
				writeln!(f, "message_received {}", hex::encode(msg.message_id()))
			},
			Self::MessageExecuted(msg) => {
				writeln!(f, "message_executed {}", hex::encode(msg))
			},
			Self::BatchExecuted { batch_id, tx_hash } => {
				let tx_hash = tx_hash.as_ref().map_or("None".to_string(), hex::encode);
				writeln!(f, "batch_executed {batch_id} with tx_hash {tx_hash}")
			},
		}
	}
}

#[cfg(feature = "std")]
use crate::TssSignature;
#[cfg(feature = "std")]
use anyhow::Result;
#[cfg(feature = "std")]
use futures::Stream;
#[cfg(feature = "std")]
use std::ops::Range;
#[cfg(feature = "std")]
use std::pin::Pin;

#[cfg(feature = "std")]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ConnectorParams {
	pub network_id: NetworkId,
	pub blockchain: String,
	pub network: String,
	pub url: String,
	pub mnemonic: String,
	pub cctp_sender: Option<String>,
	pub cctp_attestation: Option<String>,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Route {
	pub network_id: NetworkId,
	pub gateway: Gateway,
	pub relative_gas_price: (u128, u128),
	pub gas_limit: u64,
	pub base_fee: u128,
}

#[cfg(feature = "std")]
impl Route {
	pub fn relative_gas_price(&self) -> f64 {
		self.relative_gas_price.0 as f64 / self.relative_gas_price.1 as f64
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IChain: Send + Sync + 'static {
	/// Formats an address into a string.
	fn format_address(&self, address: Address) -> String;
	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address>;
	/// Returns the currency decimals and symobl.
	fn currency(&self) -> (u32, &str);
	/// Formats a balance into a string.
	fn format_balance(&self, balance: u128) -> String {
		let (decimals, symbol) = self.currency();
		crate::balance::BalanceFormatter::new(decimals, symbol).format(balance)
	}
	/// Parses a balance from a string.
	fn parse_balance(&self, balance: &str) -> Result<u128> {
		let (decimals, symbol) = self.currency();
		crate::balance::BalanceFormatter::new(decimals, symbol).parse(balance)
	}
	/// Network identifier.
	fn network_id(&self) -> NetworkId;
	/// Human readable connector account identifier.
	fn address(&self) -> Address;
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self, balance: u128) -> Result<()>;
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address, amount: u128) -> Result<()>;
	/// Queries the account balance.
	async fn balance(&self, address: Address) -> Result<u128>;
	/// Returns the last finalized block.
	async fn finalized_block(&self) -> Result<u64>;
	/// Stream of finalized block indexes.
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + 'static>>;
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IConnector: IChain {
	/// Reads gmp messages from the target chain.
	async fn read_events(&self, gateway: Gateway, blocks: Range<u64>) -> Result<Vec<GmpEvent>>;
	/// Submits a gmp message to the target chain.
	async fn submit_commands(
		&self,
		gateway: Gateway,
		batch: BatchId,
		msg: GatewayMessage,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<(), String>;
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IConnectorAdmin: IConnector {
	/// Deploys the gateway contract.
	async fn deploy_gateway(
		&self,
		additional_params: &[u8],
		proxy: &[u8],
		gateway: &[u8],
	) -> Result<(Address, u64)>;
	/// Redeploys the gateway contract.
	async fn redeploy_gateway(
		&self,
		additional_params: &[u8],
		proxy: Address,
		gateway: &[u8],
	) -> Result<()>;
	/// Returns the gateway admin.
	async fn admin(&self, gateway: Address) -> Result<Address>;
	/// Sets the gateway admin.
	async fn set_admin(&self, gateway: Address, admin: Address) -> Result<()>;
	/// Returns the registered shard keys.
	async fn shards(&self, gateway: Address) -> Result<Vec<TssPublicKey>>;
	/// Sets the registered shard keys. Overwrites any other keys.
	async fn set_shards(&self, gateway: Address, keys: &[TssPublicKey]) -> Result<()>;
	/// Returns the gateway routing table.
	async fn routes(&self, gateway: Address) -> Result<Vec<Route>>;
	/// Updates an entry in the gateway routing table.
	async fn set_route(&self, gateway: Address, route: Route) -> Result<()>;
	/// Deploys a test contract.
	async fn deploy_test(&self, gateway: Address, tester: &[u8]) -> Result<(Address, u64)>;
	/// Estimates the message gas limit.
	async fn estimate_message_gas_limit(
		&self,
		contract: Address,
		src_network: NetworkId,
		src: Address,
		payload: Vec<u8>,
	) -> Result<u128>;
	/// Estimates the message cost.
	async fn estimate_message_cost(
		&self,
		gateway: Address,
		dest_network: NetworkId,
		gas_limit: u128,
		payload: Vec<u8>,
	) -> Result<u128>;
	/// Sends a message using the test contract and returns the message id.
	async fn send_message(
		&self,
		src: Address,
		dest_network: NetworkId,
		dest: Address,
		gas_limit: u128,
		gas_cost: u128,
		payload: Vec<u8>,
	) -> Result<MessageId>;
	/// Receives messages from test contract.
	async fn recv_messages(&self, contract: Address, blocks: Range<u64>)
		-> Result<Vec<GmpMessage>>;
	/// Calculate transaction base fee for a chain.
	async fn transaction_base_fee(&self) -> Result<u128>;
	/// Calculate returns the latest block gas_limit for a chain.
	async fn block_gas_limit(&self) -> Result<u64>;
	/// Withdraw gateway funds.
	async fn withdraw_funds(&self, gateway: Address, amount: u128, address: Address) -> Result<()>;
	/// Debug a transaction.
	async fn debug_transaction(&self, _tx: Hash) -> Result<String> {
		anyhow::bail!("debugging transactions is not supported on this backend");
	}
	/// Dump anvil chain state
	async fn dump_state(&self) -> Result<String> {
		anyhow::bail!("dumping chain state is not supported on this backend");
	}
	/// Load anvil chain state
	async fn load_state(&self, _state: String) -> Result<()> {
		anyhow::bail!("loading chain state is not supported on this backend");
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IConnectorBuilder: IConnectorAdmin + Sized {
	/// Creates a new connector.
	async fn new(params: ConnectorParams) -> Result<Self>;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn boxed() {
		std::collections::HashMap::<NetworkId, Box<dyn IConnectorAdmin>>::default();
	}
}
