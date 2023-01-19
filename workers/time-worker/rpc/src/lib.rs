//! RPC API for Time Worker
#![warn(missing_docs)]
use futures::{channel::mpsc::Sender, task::SpawnError, SinkExt};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use log::warn;
use parking_lot::Mutex;
use std::sync::Arc;
use time_primitives::rpc::SignRpcPayload;
use time_worker::kv::TimeKeyvault;

#[derive(Debug, thiserror::Error)]
/// Top-level error type for the RPC handler
pub enum Error {
	/// Time RPC endpoint is not ready.
	#[error("Time RPC endpoint not ready")]
	EndpointNotReady,
	/// Time RPC background task failed to spawn.
	#[error("Time RPC background task failed to spawn")]
	RpcTaskFailure(#[from] SpawnError),
	/// Provided signature verification failed
	#[error("Provided signature verification failed")]
	SigVerificationFailure,
	/// Time key is not yet injected into node
	#[error("No time key found")]
	TimeKeyNotFound,
}

/// The error codes returned by jsonrpc.
pub enum ErrorCode {
	/// Returned when Time RPC endpoint is not ready.
	NotReady = 1,
	/// Returned on Time RPC background task failure.
	TaskFailure = 2,
	/// Returned when signature in given SignRpcPayload failed to verify
	SigFailure = 3,
	/// Returned when time key is not found
	NoTimeKey = 4,
}

impl From<Error> for ErrorCode {
	fn from(error: Error) -> Self {
		match error {
			Error::EndpointNotReady => ErrorCode::NotReady,
			Error::RpcTaskFailure(_) => ErrorCode::TaskFailure,
			Error::SigVerificationFailure => ErrorCode::SigFailure,
			Error::TimeKeyNotFound => ErrorCode::NoTimeKey,
		}
	}
}

impl From<Error> for JsonRpseeError {
	fn from(error: Error) -> Self {
		let message = error.to_string();
		let code = ErrorCode::from(error);
		JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
			code as i32,
			message,
			None::<()>,
		)))
	}
}

// Provides RPC methods for interacting with Time Worker.
#[rpc(client, server)]
pub trait TimeRpcApi {
	#[method(name = "time_submitForSigning")]
	async fn submit_for_signing(&self, payload: SignRpcPayload) -> RpcResult<()>;
}
pub struct TimeRpcApiHandler {
	// this wrapping is required by rpc boundaries
	signer: Arc<Mutex<Sender<(u64, Vec<u8>)>>>,
	kv: TimeKeyvault,
}

impl TimeRpcApiHandler {
	pub fn new(signer: Arc<Mutex<Sender<(u64, Vec<u8>)>>>, kv: TimeKeyvault) -> Self {
		Self { signer, kv }
	}
}

#[async_trait]
impl TimeRpcApi for TimeRpcApiHandler {
	async fn submit_for_signing(&self, payload: SignRpcPayload) -> RpcResult<()> {
		let keys = self.kv.public_keys();
		if keys.len() != 1 {
			Err(Error::TimeKeyNotFound)
		}
		if payload.verify(keys[0].into()) {
			self.signer.lock().send((payload.group_id, payload.message)).await?;
			Ok(())
		} else {
			Err(Error::SigVerificationFailure.into())
		}
	}
}
