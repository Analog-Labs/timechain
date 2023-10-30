use jsonrpsee::{
	core::{Error, RpcResult},
	proc_macros::rpc,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;
pub use time_primitives::{RpcShardDetails, ShardId, ShardsRpcApi as ShardsRuntimeApi};
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
}

impl<C, Block> ShardsApiServer<<Block as BlockT>::Hash> for ShardsRpcApi<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: ShardsRuntimeApi<Block>,
{
	fn get_detail(
		&self,
		shard_id: ShardId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<RpcShardDetails<BlockNumber>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		match api.get_detail(at, shard_id) {
			Ok(Some(result)) => Ok(result),
			Ok(None) => Err(Error::Custom("No shard found".into())),
			Err(e) => Err(Error::Custom(format!("Error querying storage {}", e))),
		}
	}
}
