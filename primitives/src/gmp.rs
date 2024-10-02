use crate::{NetworkId, TssPublicKey};
use scale_codec::{Decode, Encode};
use scale_info::{prelude::vec::Vec, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

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

	pub fn hash(&self, payload: &[u8]) -> Hash {
		use sha3::Digest;
		let mut hasher = sha3::Keccak256::new();
		hasher.update(GMP_VERSION);
		hasher.update(self.network.to_be_bytes());
		hasher.update(self.gateway);
		hasher.update(payload);
		hasher.finalize().into()
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
	const HEADER_LEN: usize = 113;

	pub fn encoded_len(&self) -> usize {
		Self::HEADER_LEN + self.bytes.len()
	}

	fn encode_header(&self) -> [u8; 113] {
		let mut hdr = [0; Self::HEADER_LEN];
		hdr[0] = 0; // struct version
		hdr[1..3].copy_from_slice(&self.src_network.to_be_bytes());
		hdr[3..5].copy_from_slice(&self.dest_network.to_be_bytes());
		hdr[5..37].copy_from_slice(&self.src);
		hdr[37..69].copy_from_slice(&self.dest);
		hdr[69..77].copy_from_slice(&self.nonce.to_be_bytes());
		hdr[77..93].copy_from_slice(&self.gas_limit.to_be_bytes());
		hdr[93..109].copy_from_slice(&self.gas_cost.to_be_bytes());
		hdr[109..113].copy_from_slice(&(self.bytes.len() as u32).to_be_bytes());
		hdr
	}

	pub fn encode_to(&self, buf: &mut Vec<u8>) {
		buf.extend_from_slice(&self.encode_header());
		buf.extend_from_slice(&self.bytes);
	}

	pub fn message_id(&self) -> MessageId {
		use sha3::Digest;
		let mut hasher = sha3::Keccak256::new();
		hasher.update(self.encode_header());
		hasher.update(&self.bytes);
		hasher.finalize().into()
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
	pub fn encoded_len(&self) -> usize {
		1 + match self {
			Self::SendMessage(msg) => msg.encoded_len(),
			_ => 8 + 33,
		}
	}

	pub fn encode_to(&self, buf: &mut Vec<u8>) {
		match self {
			Self::SendMessage(msg) => {
				buf.push(0);
				msg.encode_to(buf);
			},
			Self::RegisterShard(pubkey) => {
				buf.push(1);
				buf.extend_from_slice(pubkey);
			},
			Self::UnregisterShard(pubkey) => {
				buf.push(2);
				buf.extend_from_slice(pubkey);
			},
		}
	}

	pub fn gas(&self) -> u128 {
		match self {
			Self::SendMessage(msg) => msg.gas_cost,
			_ => 10_000,
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

	pub fn encode(&self, batch_id: BatchId) -> Vec<u8> {
		let mut buf = Vec::new();
		buf.push(0); // struct version
		buf.extend_from_slice(&batch_id.to_be_bytes());
		buf.extend_from_slice(&(self.ops.len() as u32).to_be_bytes());
		for op in &self.ops {
			op.encode_to(&mut buf);
		}
		buf
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
	BatchExecuted(BatchId),
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
			Self::BatchExecuted(batch) => {
				writeln!(f, "batch_executed {}", batch)
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
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Network {
	pub network_id: NetworkId,
	pub gateway: Gateway,
	pub gas_limit: u64,
	pub base_fee: u128,
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IChain: Send + Sync + 'static {
	/// Formats an address into a string.
	fn format_address(&self, address: Address) -> String;
	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address>;
	/// Formats a balance into a string.
	fn format_balance(&self, balance: u128) -> String {
		balance.to_string()
	}
	/// Parses a balance from a string.
	fn parse_balance(&self, balance: &str) -> Result<u128> {
		balance.parse().map_err(|_| anyhow::anyhow!("expected unsigned integer"))
	}
	/// Network identifier.
	fn network_id(&self) -> NetworkId;
	/// Human readable connector account identifier.
	fn address(&self) -> Address;
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self) -> Result<()>;
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address, amount: u128) -> Result<()>;
	/// Queries the account balance.
	async fn balance(&self, address: Address) -> Result<u128>;
	/// Stream of finalized block indexes.
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + 'static>>;
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IConnector: IChain {
	/// Creates a new connector.
	async fn new(params: ConnectorParams) -> Result<Self>
	where
		Self: Sized;
	/// Object-safe clone.
	fn clone(&self) -> Self
	where
		Self: Sized;
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
	async fn deploy_gateway(&self, proxy: &[u8], gateway: &[u8]) -> Result<(Address, u64)>;
	/// Redeploys the gateway contract.
	async fn redeploy_gateway(&self, proxy: Address, gateway: &[u8]) -> Result<()>;
	/// Checks if the gateway needs to be redeployed.
	async fn gateway_needs_redeployment(&self, _proxy: Address, _gateway: &[u8]) -> Result<bool> {
		Ok(false)
	}
	/// Returns the gateway admin.
	async fn admin(&self, gateway: Address) -> Result<Address>;
	/// Sets the gateway admin.
	async fn set_admin(&self, gateway: Address, admin: Address) -> Result<()>;
	/// Returns the registered shard keys.
	async fn shards(&self, gateway: Address) -> Result<Vec<TssPublicKey>>;
	/// Sets the registered shard keys. Overwrites any other keys.
	async fn set_shards(&self, gateway: Address, keys: &[TssPublicKey]) -> Result<()>;
	/// Returns the gateway routing table.
	async fn networks(&self, gateway: Address) -> Result<Vec<Network>>;
	/// Updates an entry in the gateway routing table.
	async fn set_network(&self, gateway: Address, network: Network) -> Result<()>;
	/// Deploys a test contract.
	async fn deploy_test(&self, gateway: Address, tester: &[u8]) -> Result<(Address, u64)>;
	/// Estimates the message cost.
	async fn estimate_message_cost(
		&self,
		gateway: Address,
		dest: NetworkId,
		msg_size: usize,
	) -> Result<u128>;
	/// Sends a message using the test contract.
	async fn send_message(&self, contract: Address, msg: GmpMessage) -> Result<()>;
	/// Receives messages from test contract.
	async fn recv_messages(&self, contract: Address, blocks: Range<u64>)
		-> Result<Vec<GmpMessage>>;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn boxed() {
		std::collections::HashMap::<NetworkId, Box<dyn IConnectorAdmin>>::default();
	}
}
