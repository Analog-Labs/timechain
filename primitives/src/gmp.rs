use crate::{NetworkId, ShardId, TaskId, TssPublicKey};
use scale_codec::{Decode, Encode};
use scale_decode::DecodeAsType;
use scale_info::{prelude::vec::Vec, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type Address = [u8; 32];
pub type Gateway = Address;

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct GmpParams {
	pub network: NetworkId,
	pub gateway: Gateway,
	pub signer: TssPublicKey,
}

impl GmpParams {
	pub fn domain_separator(&self) -> [u8; 32] {
		todo!()
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, Decode, DecodeAsType, Encode, TypeInfo, PartialEq)]
pub struct GmpMessage {
	pub src_network: NetworkId,
	pub src: Address,
	pub dest_network: NetworkId,
	pub dest: Address,
	pub nonce: u64,
	pub gas_limit: u128,
	pub bytes: Vec<u8>,
}

impl GmpMessage {
	pub fn message_id(&self) -> [u8; 32] {
		todo!()
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum GatewayOp {
	RegisterShard(
		ShardId,
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
	),
	UnregisterShard(
		ShardId,
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
	),
	SendMessage(GmpMessage),
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct GatewayMessage {
	pub ops: Vec<GatewayOp>,
	pub task_id: TaskId,
}

impl GatewayMessage {
	pub fn new(task_id: TaskId, ops: Vec<GatewayOp>) -> Self {
		Self { task_id, ops }
	}

	pub fn hash(&self, _params: &GmpParams) -> [u8; 32] {
		todo!()
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct Network {
	pub network_id: NetworkId,
	pub gateway: Gateway,
	pub relative_gas_price: (u128, u128),
	pub gas_limit: u64,
	pub base_fee: u128,
}

#[cfg(feature = "std")]
use crate::{TssSignature, TxHash};
#[cfg(feature = "std")]
use anyhow::Result;
#[cfg(feature = "std")]
use futures::Stream;
#[cfg(feature = "std")]
use std::num::NonZeroU64;
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};
#[cfg(feature = "std")]
use std::pin::Pin;

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IConnector {
	/// Creates a new connector.
	async fn new(
		network_id: NetworkId,
		blockchain: &str,
		network: &str,
		url: &str,
		keyfile: &Path,
	) -> Result<Self>
	where
		Self: Sized;
	/// Network identifier.
	fn network_id(&self) -> NetworkId;
	/// Human readable connector account identifier.
	fn account(&self) -> &str;
	/// Connector account balance.
	async fn balance(&self) -> Result<u128>;
	/// Reads gmp messages from the target chain.
	async fn read_messages(
		&self,
		gateway: Gateway,
		from_block: Option<NonZeroU64>,
		to_block: u64,
	) -> Result<Vec<GmpMessage>>;
	/// Submits a gmp message to the target chain.
	async fn submit_messages(
		&self,
		gateway: Gateway,
		msg: GatewayMessage,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<Vec<TxHash>, String>;
	/// Verifies the submission of a gmp message and returns a proof.
	async fn verify_submission(&self, msg: GatewayMessage, tx: Vec<TxHash>) -> Result<Vec<u8>>;
	/// Stream of finalized block indexes.
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>>;
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IConnectorAdmin {
	type Connector: IConnector;

	/// Creates a new admin connector from a connector and some smart contracts.
	fn new(connector: Self::Connector, gateway: PathBuf, proxy: PathBuf, tester: PathBuf) -> Self
	where
		Self: Sized;
	/// Deploys the gateway contract.
	async fn deploy_gateway(&self) -> Result<(Gateway, u64)>;
	/// Redeploys the gateway contract.
	async fn redeploy_gateway(&self, gateway: Gateway) -> Result<()>;
	/// Returns the gateway admin.
	async fn admin(&self, gateway: Gateway) -> Result<Address>;
	/// Sets the gateway admin.
	async fn set_admin(&self, gateway: Gateway, admin: Address) -> Result<()>;
	/// Returns the registered shard keys.
	async fn shards(&self, gateway: Gateway) -> Result<Vec<TssPublicKey>>;
	/// Sets the registered shard keys. Overwrites any other keys.
	async fn set_shards(&self, gateway: Gateway, keys: &[TssPublicKey]) -> Result<()>;
	/// Returns the gateway routing table.
	async fn networks(&self) -> Result<Vec<Network>>;
	/// Updates an entry in the gateway routing table.
	async fn set_network(&self, gateway: Gateway, network: Network) -> Result<()>;
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait IConnectorTest: IConnectorAdmin {
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self) -> Result<()>;
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address, amount: u128) -> Result<()>;
	/// Deploys a test contract.
	async fn deploy_test(&self, gateway: Gateway) -> Result<(Address, u64)>;
	/// Estimates the message cost.
	async fn estimate_message_cost(
		&self,
		gateway: Gateway,
		dest: NetworkId,
		msg_size: usize,
	) -> Result<u128>;
	/// Sends a message using the test contract.
	async fn send_message(&self, contract: Address, msg: GmpMessage) -> Result<()>;
	/// Receives messages from test contract.
	async fn recv_messages(
		&self,
		contract: Address,
		from_block: Option<NonZeroU64>,
		to_block: u64,
	) -> Result<Vec<GmpMessage>>;
}
