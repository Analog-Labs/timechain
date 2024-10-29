use serde::{Deserialize, Serialize};
use serde_big_array::Array;
use time_primitives::{
	Address, BatchId, Gateway, GatewayMessage, GmpEvent, GmpMessage, NetworkId, Route,
	TssPublicKey, TssSignature,
};

#[derive(Serialize, Deserialize)]
pub struct FaucetRequest {}

#[derive(Serialize, Deserialize)]
pub struct FaucetResponse {}

#[derive(Serialize, Deserialize)]
pub struct TransferRequest {
	pub address: Address,
	pub amount: u128,
}

#[derive(Serialize, Deserialize)]
pub struct TransferResponse {}

#[derive(Serialize, Deserialize)]
pub struct BalanceRequest {
	pub address: Address,
}

#[derive(Serialize, Deserialize)]
pub struct BalanceResponse {
	pub balance: u128,
}

#[derive(Serialize, Deserialize)]
pub struct BlockStreamRequest {}

#[derive(Serialize, Deserialize)]
pub struct BlockStreamResponse {
	pub block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ReadEventsRequest {
	pub gateway: Gateway,
	pub start_block: u64,
	pub end_block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ReadEventsResponse {
	pub events: Vec<GmpEvent>,
}

#[derive(Serialize, Deserialize)]
pub struct SubmitCommandsRequest {
	pub gateway: Gateway,
	pub batch: BatchId,
	pub msg: GatewayMessage,
	#[serde(with = "time_primitives::serde_tss_public_key")]
	pub signer: TssPublicKey,
	#[serde(with = "time_primitives::serde_tss_signature")]
	pub sig: TssSignature,
}

#[derive(Serialize, Deserialize)]
pub struct SubmitCommandsResponse {}

#[derive(Serialize, Deserialize)]
pub struct DeployGatewayRequest {
	pub proxy: Vec<u8>,
	pub gateway: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct DeployGatewayResponse {
	pub address: Address,
	pub block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct RedeployGatewayRequest {
	pub proxy: Address,
	pub gateway: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct RedeployGatewayResponse {}

#[derive(Serialize, Deserialize)]
pub struct AdminRequest {
	pub gateway: Gateway,
}

#[derive(Serialize, Deserialize)]
pub struct AdminResponse {
	pub address: Address,
}

#[derive(Serialize, Deserialize)]
pub struct SetAdminRequest {
	pub gateway: Gateway,
	pub admin: Address,
}

#[derive(Serialize, Deserialize)]
pub struct SetAdminResponse {}

#[derive(Serialize, Deserialize)]
pub struct ShardsRequest {
	pub gateway: Gateway,
}

#[derive(Serialize, Deserialize)]
pub struct ShardsResponse {
	pub shards: Vec<Array<u8, 33>>,
}

#[derive(Serialize, Deserialize)]
pub struct SetShardsRequest {
	pub gateway: Gateway,
	pub shards: Vec<Array<u8, 33>>,
}

#[derive(Serialize, Deserialize)]
pub struct SetShardsResponse {}

#[derive(Serialize, Deserialize)]
pub struct RoutesRequest {
	pub gateway: Gateway,
}

#[derive(Serialize, Deserialize)]
pub struct RoutesResponse {
	pub routes: Vec<Route>,
}

#[derive(Serialize, Deserialize)]
pub struct SetRouteRequest {
	pub gateway: Gateway,
	pub route: Route,
}

#[derive(Serialize, Deserialize)]
pub struct SetRouteResponse {}

#[derive(Serialize, Deserialize)]
pub struct DeployTestRequest {
	pub gateway: Address,
	pub tester: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct DeployTestResponse {
	pub address: Address,
	pub block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EstimateMessageCostRequest {
	pub gateway: Gateway,
	pub dest: NetworkId,
	pub msg_size: usize,
}

#[derive(Serialize, Deserialize)]
pub struct EstimateMessageCostResponse {
	pub cost: u128,
}

#[derive(Serialize, Deserialize)]
pub struct SendMessageRequest {
	pub contract: Address,
	pub msg: GmpMessage,
}

#[derive(Serialize, Deserialize)]
pub struct SendMessageResponse {}

#[derive(Serialize, Deserialize)]
pub struct RecvMessagesRequest {
	pub contract: Address,
	pub start_block: u64,
	pub end_block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct RecvMessagesResponse {
	pub messages: Vec<GmpMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct TransactionBaseFeeRequest {}

#[derive(Serialize, Deserialize)]
pub struct TransactionBaseFeeResponse {
	pub base_fee: u128,
}
