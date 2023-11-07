use jsonrpsee::{
	core::{Error, RpcResult},
	proc_macros::rpc,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;
use time_primitives::{
	AccountId, Commitment, MemberStatus, RpcShardDetails, SerializedMemberStatus, ShardId,
	ShardsApi,
};
type BlockNumber = u32;
#[rpc(client, server)]
pub trait ShardsApi<BlockHash> {
	#[method(name = "shards_getDetail")]
	fn get_detail(
		&self,
		shard_id: ShardId,
		at: Option<BlockHash>,
	) -> RpcResult<RpcShardDetails<BlockNumber>>;
}

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
	#[error("Error querying runtime")]
	ErrorQueryingRuntime,
	#[error("ShardId '{0}' not found")]
	ShardNotFound(ShardId),
}
impl From<RpcError> for Error {
	fn from(value: RpcError) -> Self {
		match value {
			RpcError::ErrorQueryingRuntime => Error::to_call_error(RpcError::ErrorQueryingRuntime),
			RpcError::ShardNotFound(shard_id) => {
				Error::to_call_error(RpcError::ShardNotFound(shard_id))
			},
		}
	}
}
pub struct ShardsRpcApi<C, Block> {
	_block: std::marker::PhantomData<Block>,
	client: Arc<C>,
}
impl<C, Block> ShardsRpcApi<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			_block: Default::default(),
			client,
		}
	}

	fn convert_member_status(status: MemberStatus) -> SerializedMemberStatus {
		match status {
			MemberStatus::Added => SerializedMemberStatus::Added,
			MemberStatus::Committed(commitment) => {
				SerializedMemberStatus::Committed(Self::serialize_commitment(commitment))
			},
			MemberStatus::Ready => SerializedMemberStatus::Ready,
		}
	}

	fn serialize_commitment(commitment: Commitment) -> Vec<String> {
		commitment.iter().map(|hash| format!("0x{}", hex::encode(hash))).collect()
	}
}

impl<C, Block> ShardsApiServer<<Block as BlockT>::Hash> for ShardsRpcApi<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: ShardsApi<Block>,
{
	fn get_detail(
		&self,
		shard_id: ShardId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<RpcShardDetails<BlockNumber>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		let shard_status =
			api.get_shard_status(at, shard_id).map_err(|_| RpcError::ErrorQueryingRuntime)?;
		let shard_threshold = api
			.get_shard_threshold(at, shard_id)
			.map_err(|_| RpcError::ErrorQueryingRuntime)?;
		let shard_members: Vec<(AccountId, SerializedMemberStatus)> = api
			.get_shard_members(at, shard_id)
			.map_err(|_| RpcError::ErrorQueryingRuntime)?
			.iter()
			.map(|(id, status)| (id.clone(), Self::convert_member_status(status.clone())))
			.collect();
		let shard_commitment: Vec<String> = Self::serialize_commitment(
			api.get_shard_commitment(at, shard_id)
				.map_err(|_| RpcError::ErrorQueryingRuntime)?,
		);
		Ok(RpcShardDetails::new(shard_status, shard_threshold, shard_members, shard_commitment))
	}
}
