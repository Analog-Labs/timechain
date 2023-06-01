//! RPC API for Time Worker
#![allow(clippy::type_complexity)]
use futures::{channel::mpsc, task::SpawnError, SinkExt};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use log::info;
use sp_keystore::KeystorePtr;
use time_primitives::{rpc::SignRpcPayload, KEY_TYPE};

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
	async fn submit_for_signing(
		&self,
		group_id: u64,
		message: String,
		task_id: u64,
		signature: String,
	) -> RpcResult<()>;
}

pub struct TimeRpcApiHandler {
	// this wrapping is required by rpc boundaries
	signer: mpsc::Sender<(u64, u64, [u8; 32])>,
	kv: KeystorePtr,
}

impl TimeRpcApiHandler {
	/// Constructor takes channel of TSS set id and hash of the event to sign
	pub fn new(signer: mpsc::Sender<(u64, u64, [u8; 32])>, kv: KeystorePtr) -> Self {
		Self { signer, kv }
	}
}

#[async_trait]
impl TimeRpcApiServer for TimeRpcApiHandler {
	async fn submit_for_signing(
		&self,
		group_id: u64,
		message: String,
		task_id: u64,
		signature: String,
	) -> RpcResult<()> {
		info!("data received ===> message == {:?}  |  signature == {:?}", message, signature);
		if let Ok(message) = serde_json::from_str::<[u8; 32]>(&message) {
			if let Ok(signature) = serde_json::from_str::<Vec<u8>>(&signature) {
				let signature: [u8; 64] = arrayref::array_ref!(signature, 0, 64).to_owned();
				let keys = self.kv.sr25519_public_keys(KEY_TYPE);
				if keys.len() != 1 {
					return Err(Error::TimeKeyNotFound.into());
				}
				let payload = SignRpcPayload::new(group_id, message, signature);
				if payload.verify(keys[0].into()) {
					self.signer.clone().send((payload.group_id, task_id, message)).await?;
					Ok(())
				} else {
					Err(Error::SigVerificationFailure.into())
				}
			} else {
				return Err(Error::SigVerificationFailure.into());
			}
		} else {
			return Err(Error::SigVerificationFailure.into());
		}
	}
}
