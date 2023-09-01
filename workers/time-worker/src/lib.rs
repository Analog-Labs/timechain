#![allow(clippy::type_complexity)]
pub mod tx_submitter;
mod worker;

#[cfg(test)]
mod tests;

use futures::channel::mpsc;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network::config::{IncomingRequest, RequestResponseConfig};
use sc_network::NetworkRequest;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc, time::Duration};
use time_primitives::{
	MembersApi, PeerId, PublicKey, ShardsApi, SubmitMembers, SubmitShards, TaskExecutor, TssRequest,
};

/// Constant to indicate target for logging
pub const TW_LOG: &str = "time-worker";

/// time protocol name suffix.
pub const PROTOCOL_NAME: &str = "/time/1";

pub fn protocol_config(tx: async_channel::Sender<IncomingRequest>) -> RequestResponseConfig {
	RequestResponseConfig {
		name: PROTOCOL_NAME.into(),
		fallback_names: vec![],
		max_request_size: 1024 * 1024,
		max_response_size: 0,
		request_timeout: Duration::from_secs(3),
		inbound_queue: Some(tx),
	}
}

/// Set of properties we need to run our gadget
pub struct TimeWorkerParams<B: Block, C, R, N, T, TxSub>
where
	B: Block + 'static,
	C: BlockchainEvents<B> + HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: MembersApi<B> + ShardsApi<B>,
	N: NetworkRequest,
	T: TaskExecutor<B>,
	TxSub: SubmitShards<B> + SubmitMembers<B>,
{
	pub _block: PhantomData<B>,
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub network: N,
	pub task_executor: T,
	pub tx_submitter: TxSub,
	pub public_key: PublicKey,
	pub peer_id: PeerId,
	pub tss_request: mpsc::Receiver<TssRequest>,
	pub protocol_request: async_channel::Receiver<IncomingRequest>,
}

/// Start the Timeworker gadget.
///
/// This is a thin shim around running and awaiting a time worker.
pub async fn start_timeworker_gadget<B, C, R, N, T, TxSub>(
	timeworker_params: TimeWorkerParams<B, C, R, N, T, TxSub>,
) where
	B: Block + 'static,
	C: BlockchainEvents<B> + HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: MembersApi<B> + ShardsApi<B>,
	N: NetworkRequest,
	T: TaskExecutor<B>,
	TxSub: SubmitShards<B> + SubmitMembers<B>,
{
	let TimeWorkerParams {
		_block,
		client,
		runtime,
		network,
		task_executor,
		tx_submitter,
		public_key,
		peer_id,
		tss_request,
		protocol_request,
	} = timeworker_params;
	let worker_params = worker::WorkerParams {
		_block,
		client,
		runtime,
		network,
		task_executor,
		tx_submitter,
		public_key,
		peer_id,
		tss_request,
		protocol_request,
	};
	let mut worker = worker::TimeWorker::new(worker_params);
	worker.run().await
}
