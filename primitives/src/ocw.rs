use crate::Vec;
use crate::{CycleStatus, ShardId, TaskCycle, TaskError, TaskId, TssPublicKey};
use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
use sp_runtime::offchain::{OffchainStorage, STORAGE_PREFIX};

pub const OCW_LOCK: &[u8] = b"ocwlock";
pub const OCW_READ_ID: &[u8] = b"ocwreadid";
pub const OCW_WRITE_ID: &[u8] = b"ocwwriteid";
pub const OCW_MESSAGE_PREFIX: &[u8] = b"ocwmsg";

pub fn msg_key(id: u64) -> [u8; 14] {
	let mut key = [0; 14];
	key[..6].copy_from_slice(OCW_MESSAGE_PREFIX);
	key[6..].copy_from_slice(&id.to_be_bytes());
	key
}

#[derive(Clone, Debug, PartialEq, Decode, Encode, TypeInfo)]
pub enum OcwPayload {
	SubmitTssPublicKey { shard_id: ShardId, public_key: TssPublicKey },
	SubmitTaskHash { shard_id: ShardId, task_id: TaskId, hash: String },
	SubmitTaskResult { task_id: TaskId, cycle: TaskCycle, status: CycleStatus },
	SubmitTaskError { task_id: TaskId, error: TaskError },
}

impl OcwPayload {
	pub fn shard_id(&self) -> ShardId {
		match self {
			Self::SubmitTssPublicKey { shard_id, .. } => *shard_id,
			Self::SubmitTaskHash { shard_id, .. } => *shard_id,
			Self::SubmitTaskResult { status, .. } => status.shard_id,
			Self::SubmitTaskError { error, .. } => error.shard_id,
		}
	}
}

pub fn write_message_with_prefix<B: OffchainStorage>(
	mut storage: B,
	prefix: &[u8],
	payload: &OcwPayload,
) {
	let payload = payload.encode();
	loop {
		let raw_id = storage.get(prefix, OCW_WRITE_ID);
		let id = raw_id
			.as_deref()
			.map(|mut id| u64::decode(&mut id).unwrap())
			.unwrap_or_default();
		if !storage.compare_and_set(prefix, OCW_WRITE_ID, raw_id.as_deref(), &(id + 1).encode()) {
			continue;
		}
		storage.set(prefix, &msg_key(id), &payload);
		break;
	}
}

pub fn write_message<B: OffchainStorage>(storage: B, payload: &OcwPayload) {
	write_message_with_prefix(storage, STORAGE_PREFIX, payload);
}

/// Only used for testing.
pub fn read_message_with_prefix<B: OffchainStorage>(
	mut storage: B,
	prefix: &[u8],
) -> Option<OcwPayload> {
	loop {
		let raw_read_id = storage.get(prefix, OCW_READ_ID);
		let read_id = raw_read_id
			.as_deref()
			.map(|mut id| u64::decode(&mut id).unwrap())
			.unwrap_or_default();
		let write_id = storage
			.get(prefix, OCW_WRITE_ID)
			.as_deref()
			.map(|mut id| u64::decode(&mut id).unwrap())
			.unwrap_or_default();
		if read_id >= write_id {
			return None;
		}
		if !storage.compare_and_set(
			prefix,
			OCW_READ_ID,
			raw_read_id.as_deref(),
			&(read_id + 1).encode(),
		) {
			continue;
		}
		let msg_key = msg_key(read_id);
		let raw_msg = storage.get(prefix, &msg_key).unwrap();
		let msg = OcwPayload::decode(&mut &raw_msg[..]).unwrap();
		return Some(msg);
	}
}

/// Only used for testing.
pub fn read_message<B: OffchainStorage>(storage: B) -> Option<OcwPayload> {
	read_message_with_prefix(storage, STORAGE_PREFIX)
}

pub fn submit_transaction(data: Vec<u8>) {
	let _result = sp_io::offchain::submit_transaction(data);
}
