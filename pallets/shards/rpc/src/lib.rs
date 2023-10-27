use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;
pub use time_primitives::{ShardId, ShardsRpcApi as ShardsRuntimeApi};
#[rpc(client, server)]
pub trait ShardsApi<BlockHash> {
	#[method(name = "shards_getDetail")]
	fn get_detail(&self, at: Option<BlockHash>, shard_id: ShardId) -> RpcResult<u64>;
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
		at: Option<<Block as BlockT>::Hash>,
		_shard_id: ShardId,
	) -> RpcResult<u64> {
		let _api = self.client.runtime_api();
		let _at = at.unwrap_or_else(|| self.client.info().best_hash);
		// todo shard rpc logic
		Ok(0)
	}
}
