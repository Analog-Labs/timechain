use serde::{Deserialize, Serialize};
use serde_big_array::Array;
use time_primitives::{
	Address32, BatchId, GatewayMessage, GmpEvent, GmpMessage, Hash, MessageId, NetworkId, Route,
	TssPublicKey, TssSignature,
};

#[derive(Serialize, Deserialize)]
pub struct FaucetRequest {
	pub balance: u128,
}

#[derive(Serialize, Deserialize)]
pub struct FaucetResponse {}

#[derive(Serialize, Deserialize)]
pub struct TransferRequest {
	pub address: Address32,
	pub amount: u128,
}

#[derive(Serialize, Deserialize)]
pub struct TransferResponse {}

#[derive(Serialize, Deserialize)]
pub struct BalanceRequest {
	pub address: Address32,
}

#[derive(Serialize, Deserialize)]
pub struct BalanceResponse {
	pub balance: u128,
}

#[derive(Serialize, Deserialize)]
pub struct FinalizedBlockRequest {}

#[derive(Serialize, Deserialize)]
pub struct FinalizedBlockResponse {
	pub finalized_block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct BlockStreamRequest {}

#[derive(Serialize, Deserialize)]
pub struct BlockStreamResponse {
	pub block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ReadEventsRequest {
	pub gateway: Address32,
	pub start_block: u64,
	pub end_block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ReadEventsResponse {
	pub events: Vec<GmpEvent>,
}

#[derive(Serialize, Deserialize)]
pub struct SubmitCommandsRequest {
	pub gateway: Address32,
	pub batch: BatchId,
	pub msg: GatewayMessage,
	pub gas_price: u128,
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
	pub address: Address32,
	pub block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct RedeployGatewayRequest {
	pub proxy: Address32,
	pub gateway: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct RedeployGatewayResponse {}

#[derive(Serialize, Deserialize)]
pub struct AdminRequest {
	pub gateway: Address32,
}

#[derive(Serialize, Deserialize)]
pub struct AdminResponse {
	pub address: Address32,
}

#[derive(Serialize, Deserialize)]
pub struct SetAdminRequest {
	pub gateway: Address32,
	pub admin: Address32,
}

#[derive(Serialize, Deserialize)]
pub struct SetAdminResponse {}

#[derive(Serialize, Deserialize)]
pub struct ShardsRequest {
	pub gateway: Address32,
}

#[derive(Serialize, Deserialize)]
pub struct ShardsResponse {
	pub shards: Vec<Array<u8, 33>>,
}

#[derive(Serialize, Deserialize)]
pub struct SetShardsRequest {
	pub gateway: Address32,
	pub register: Vec<(Array<u8, 33>, u16)>,
	pub revoke: Vec<(Array<u8, 33>, u16)>,
}

#[derive(Serialize, Deserialize)]
pub struct SetShardsResponse {}

#[derive(Serialize, Deserialize)]
pub struct RoutesRequest {
	pub gateway: Address32,
}

#[derive(Serialize, Deserialize)]
pub struct RoutesResponse {
	pub routes: Vec<Route>,
}

#[derive(Serialize, Deserialize)]
pub struct SetRouteRequest {
	pub gateway: Address32,
	pub route: Route,
}

#[derive(Serialize, Deserialize)]
pub struct SetRouteResponse {}

#[derive(Serialize, Deserialize)]
pub struct DeployTesterRequest {
	pub gateway: Address32,
	pub tester: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct DeployTesterResponse {
	pub address: Address32,
	pub block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EstimateMessageGasLimitRequest {
	pub contract: Address32,
	pub src_network: NetworkId,
	pub src: Address32,
	pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct EstimateMessageGasLimitResponse {
	pub gas_limit: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EstimateMessageCostRequest {
	pub gateway: Address32,
	pub dest_network: NetworkId,
	pub msg_size: u16,
	pub gas_limit: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EstimateMessageCostResponse {
	pub cost: u128,
}

#[derive(Serialize, Deserialize)]
pub struct SendMessageRequest {
	pub src: Address32,
	pub dest_network: NetworkId,
	pub dest: Address32,
	pub gas_limit: u64,
	pub msg_cost: u128,
	pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct SendMessageResponse {
	pub message_id: MessageId,
}

#[derive(Serialize, Deserialize)]
pub struct RecvMessagesRequest {
	pub contract: Address32,
	pub start_block: u64,
	pub end_block: u64,
}

#[derive(Serialize, Deserialize)]
pub struct RecvMessagesResponse {
	pub messages: Vec<GmpMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct GasPriceRequest {}

#[derive(Serialize, Deserialize)]
pub struct GasPriceResponse {
	pub fee: u128,
}

#[derive(Serialize, Deserialize)]
pub struct BlockGasLimitRequest {}

#[derive(Serialize, Deserialize)]
pub struct BlockGasLimitResponse {
	pub gas_limit: u64,
}

#[derive(Serialize, Deserialize)]
pub struct WithdrawFundsRequest {
	pub gateway: Address32,
	pub amount: u128,
	pub address: Address32,
}

#[derive(Serialize, Deserialize)]
pub struct WithdrawFundsResponse {}

#[derive(Serialize, Deserialize)]
pub struct DebugTransactionRequest {
	pub tx: Hash,
}

#[derive(Serialize, Deserialize)]
pub struct DebugTransactionResponse {
	pub details: String,
}
