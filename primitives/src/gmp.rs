use crate::{NetworkId, TssPublicKey};
#[cfg(feature = "std")]
use crate::{TssSignature, U256};
#[cfg(feature = "std")]
use anyhow::Result;
use scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::{prelude::vec::Vec, TypeInfo};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
#[cfg(feature = "std")]
use std::ops::Range;

pub type Address32 = [u8; 32];
pub type MessageId = [u8; 32];
pub type Hash = [u8; 32];
pub type BatchId = u64;

const GMP_VERSION: &str = "Analog GMP v3";

pub trait FixedSizeEncodable {
	fn left_pad_32(&self) -> [u8; 32];
}

macro_rules! impl_fixed_size_encodable {
    ($($n:expr),*) => {
        $(
            impl FixedSizeEncodable for [u8; $n] {
                fn left_pad_32(&self) -> [u8; 32] {
                    let mut out = [0u8; 32];
                    out[32-$n..].copy_from_slice(self);
                    out
                }
            }
        )*
    }
}

impl_fixed_size_encodable!(
	0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
	26, 27, 28, 29, 30, 31, 32
);

#[derive(Debug, Clone, Decode, DecodeWithMemTracking, Encode, TypeInfo, PartialEq)]
pub struct GmpParams {
	pub network: NetworkId,
	pub gateway: Address32,
}

impl GmpParams {
	pub fn new(network: NetworkId, gateway: Address32) -> Self {
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
#[derive(
	Debug,
	Clone,
	Default,
	Decode,
	DecodeWithMemTracking,
	Encode,
	TypeInfo,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
)]
pub struct GmpMessage {
	pub src_network: NetworkId,
	pub dest_network: NetworkId,
	pub src: Address32,
	pub dest: Address32,
	pub nonce: u64,
	pub gas_limit: u128,
	/// deprecated field
	pub gas_cost: u128,
	pub bytes: Vec<u8>,
}

impl GmpMessage {
	const HEADER_LEN: usize = 224;

	pub fn encoded_len(&self) -> usize {
		Self::HEADER_LEN + self.bytes.len()
	}

	fn encode_header(&self) -> [u8; 224] {
		let mut hdr = [0u8; 224];
		hdr[32..64].copy_from_slice(&self.src.left_pad_32());
		hdr[64..96].copy_from_slice(&self.src_network.to_be_bytes().left_pad_32());
		hdr[96..128].copy_from_slice(&self.dest.left_pad_32());
		hdr[128..160].copy_from_slice(&self.dest_network.to_be_bytes().left_pad_32());
		hdr[160..192].copy_from_slice(&(self.gas_limit as u64).to_be_bytes().left_pad_32());
		hdr[192..224].copy_from_slice(&self.nonce.to_be_bytes().left_pad_32());
		hdr
	}

	pub fn message_id(&self) -> MessageId {
		let header = self.encode_header();
		Keccak256::digest(header).into()
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for GmpMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&hex::encode(self.message_id()))
	}
}

#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	Decode,
	DecodeWithMemTracking,
	Encode,
	TypeInfo,
	PartialEq,
	Eq,
	Serialize,
	Deserialize,
)]
pub struct BatchGasParams {
	pub batch_gas_limit: u64,
	pub batch_exec_gas: u64,
	pub reg_op_exec_gas: u64,
	pub unreg_op_exec_gas: u64,
	pub msg_op_exec_gas: u64,
	pub msg_byte_gas: u64,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, DecodeWithMemTracking, Encode, TypeInfo, PartialEq)]
pub enum GatewayOp {
	SendMessage(GmpMessage),
	RegisterShard(
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
		u16,
	),
	UnregisterShard(
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
		u16,
	),
}

impl GatewayOp {
	fn code(&self) -> u8 {
		match self {
			GatewayOp::SendMessage(_) => 1,
			GatewayOp::RegisterShard(_, _) => 2,
			GatewayOp::UnregisterShard(_, _) => 3,
		}
	}

	fn hash(&self) -> [u8; 32] {
		let mut bytes = [0; 96];
		match self {
			Self::SendMessage(msg) => {
				let data = Keccak256::digest(&msg.bytes);
				bytes[..32].copy_from_slice(&msg.message_id());
				bytes[32..64].copy_from_slice(&data);
				return Keccak256::digest(&bytes[..64]).into();
			},
			Self::RegisterShard(pubkey, sessions) => {
				bytes[31..64].copy_from_slice(pubkey);
				bytes[64..96].copy_from_slice(&sessions.to_be_bytes().left_pad_32());
			},
			Self::UnregisterShard(pubkey, sessions) => {
				bytes[31..64].copy_from_slice(pubkey);
				bytes[64..96].copy_from_slice(&sessions.to_be_bytes().left_pad_32());
			},
		}
		Keccak256::digest(bytes).into()
	}

	pub fn gas(&self, params: &BatchGasParams) -> u64 {
		match self {
			Self::SendMessage(msg) => {
				params.msg_op_exec_gas
					+ msg.bytes.len() as u64 * params.msg_byte_gas
					+ msg.gas_limit as u64
			},
			Self::RegisterShard(_, _) => params.reg_op_exec_gas,
			Self::UnregisterShard(_, _) => params.unreg_op_exec_gas,
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
			Self::RegisterShard(key, sessions) => {
				writeln!(f, "register_shard {} {}", hex::encode(key), sessions)
			},
			Self::UnregisterShard(key, sessions) => {
				writeln!(f, "unregister_shard {} {}", hex::encode(key), sessions)
			},
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, DecodeWithMemTracking, Encode, TypeInfo, PartialEq)]
pub struct GatewayMessage {
	pub ops: Vec<GatewayOp>,
}

impl GatewayMessage {
	pub fn new(ops: Vec<GatewayOp>) -> Self {
		Self { ops }
	}

	pub fn hash(&self, batch_id: BatchId) -> [u8; 32] {
		let mut ops_hash = [0; 32];
		for op in &self.ops {
			let mut ops_hasher = Keccak256::new();
			ops_hasher.update(ops_hash);

			let mut op_code = [0; 32];
			op_code[31] = op.code();
			ops_hasher.update(op_code);

			let op_hash = op.hash();
			ops_hasher.update(op_hash);

			ops_hash = ops_hasher.finalize().into();
		}

		let mut buf = [0; 96];
		// include version in buffer
		buf[..32].copy_from_slice(&[0u8; 32]);
		// include batch id padded to uint256
		buf[32..64].copy_from_slice(&batch_id.to_be_bytes().left_pad_32());
		buf[64..].copy_from_slice(&ops_hash);
		Keccak256::digest(buf).into()
	}

	pub fn gas(&self, params: &BatchGasParams) -> u64 {
		self.ops.iter().fold(0u64, |acc, op| acc.saturating_add(op.gas(params)))
	}
}

pub struct BatchBuilder {
	params: BatchGasParams,
	gas: u64,
	ops: Vec<GatewayOp>,
}

impl BatchBuilder {
	pub fn new(params: BatchGasParams) -> Self {
		Self {
			gas: params.batch_exec_gas,
			params,
			ops: Default::default(),
		}
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
		let gas = op.gas(&self.params);
		let batch =
			if self.gas + gas > self.params.batch_gas_limit { self.take_batch() } else { None };
		self.ops.push(op);
		batch
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(
	Debug, Clone, Decode, DecodeWithMemTracking, Encode, TypeInfo, Eq, PartialEq, Ord, PartialOrd,
)]
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
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ConnectorParams {
	pub network_id: NetworkId,
	pub url: String,
	pub mnemonic: String,
	pub chain_dict: Vec<u8>,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Route {
	/// Destination network Id
	pub network_id: NetworkId,
	/// Destination gateway
	pub gateway: Address32,
	/// Gas price on destination network, expressed in source network token
	pub relative_gas_price: (U256, U256),
	/// Maximum amount of gas a message is allowed to spend on destination network
	pub gas_limit: u64,
	/// GMP protocol fee for message delivery to the destination network, expressed in source network token
	pub gmp_base_fee: u128,
	/// Base gas coefficient.
	pub base_gas: u64,
	/// Gas per message byte.
	pub msg_byte_gas: u64,
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IChain: Send + Sync + 'static {
	/// Formats an address into a string.
	fn format_address(&self, address: Address32) -> String;
	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address32>;
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
	fn address(&self) -> Address32;
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self, balance: u128) -> Result<()>;
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address32, amount: u128) -> Result<()>;
	/// Queries the account balance.
	async fn balance(&self, address: Address32) -> Result<u128>;
	/// Returns the last finalized block.
	async fn finalized_block(&self) -> Result<u64>;
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IConnector: IChain {
	/// Reads gmp messages from the target chain.
	async fn read_events(&self, gateway: Address32, blocks: Range<u64>) -> Result<Vec<GmpEvent>>;
	/// Submits a gmp message to the target chain.
	async fn submit_commands(
		&self,
		gateway: Address32,
		batch: BatchId,
		msg: GatewayMessage,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<(), String>;
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IConnectorAdmin: IConnector {
	/// Deploys the proxy contract.
	async fn deploy_gateway(&self, proxy: &[u8], gateway: &[u8]) -> Result<(Address32, u64)>;
	/// Redeploys the gateway contract.
	async fn redeploy_gateway(&self, proxy: Address32, gateway: &[u8]) -> Result<()>;
	/// Returns the gateway admin.
	async fn admin(&self, gateway: Address32) -> Result<Address32>;
	/// Sets the gateway admin.
	async fn set_admin(&self, gateway: Address32, admin: Address32) -> Result<()>;
	/// Returns the registered shard keys.
	async fn shards(&self, gateway: Address32) -> Result<Vec<TssPublicKey>>;
	/// Sets the registered shard keys. Overwrites any other keys.
	async fn set_shards(
		&self,
		gateway: Address32,
		register: &[(TssPublicKey, u16)],
		revoke: &[(TssPublicKey, u16)],
	) -> Result<()>;
	/// Returns the gateway routing table.
	async fn routes(&self, gateway: Address32) -> Result<Vec<Route>>;
	/// Updates an entry in the gateway routing table.
	async fn set_route(&self, gateway: Address32, route: Route) -> Result<()>;
	/// Deploys a test contract.
	async fn deploy_tester(&self, gateway: Address32, tester: &[u8]) -> Result<(Address32, u64)>;
	/// Estimates the message gas limit.
	async fn estimate_message_gas_limit(
		&self,
		contract: Address32,
		src_network: NetworkId,
		src: Address32,
		payload: Vec<u8>,
	) -> Result<u64>;
	/// Estimates the message cost.
	async fn estimate_message_cost(
		&self,
		gateway: Address32,
		dest_network: NetworkId,
		msg_size: u16,
		gas_limit: u64,
	) -> Result<u128>;
	/// Sends a message using the test contract and returns the message id.
	async fn send_message(
		&self,
		src: Address32,
		dest_network: NetworkId,
		dest: Address32,
		gas_limit: u64,
		msg_cost: u128,
		payload: Vec<u8>,
	) -> Result<MessageId>;
	/// Receives messages from test contract.
	async fn recv_messages(
		&self,
		contract: Address32,
		blocks: Range<u64>,
	) -> Result<Vec<GmpMessage>>;
	/// Get EIP1559 `max_fee_per_gas` estimate for a chain.
	async fn max_fee_per_gas(&self) -> Result<u128>;
	/// Calculate returns the latest block gas_limit for a chain.
	async fn block_gas_limit(&self) -> Result<u64>;
	/// Withdraw gateway funds.
	async fn withdraw_funds(
		&self,
		gateway: Address32,
		amount: u128,
		address: Address32,
	) -> Result<()>;
	/// Debug a transaction.
	async fn debug_transaction(&self, _tx: Hash) -> Result<String>;
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
