use crate::{ScheduleCycle, ScheduleStatus, ShardId, TaskId, TssPublicKey};
use codec::{Decode, Encode};
use sp_runtime::offchain::{OffchainStorage, STORAGE_PREFIX};

pub const OCW_READ_ID: &[u8] = b"ocwreadid";
pub const OCW_WRITE_ID: &[u8] = b"ocwwriteid";
pub const OCW_MESSAGE_PREFIX: &[u8] = b"ocwmsg";

pub fn msg_key(id: u64) -> [u8; 14] {
	let mut key = [0; 14];
	key[..6].copy_from_slice(OCW_MESSAGE_PREFIX);
	key[6..].copy_from_slice(&id.to_be_bytes());
	key
}

#[derive(Clone, Decode, Encode)]
pub enum OcwPayload {
	SubmitTssPublicKey { shard_id: ShardId, public_key: TssPublicKey },
	SubmitTaskResult { task_id: TaskId, cycle: ScheduleCycle, status: ScheduleStatus },
}

impl OcwPayload {
	pub fn shard_id(&self) -> ShardId {
		match self {
			Self::SubmitTssPublicKey { shard_id, .. } => *shard_id,
			Self::SubmitTaskResult { status, .. } => status.shard_id,
		}
	}
}

pub fn write_message<B: OffchainStorage>(mut storage: B, payload: &OcwPayload) {
	let payload = payload.encode();
	loop {
		let raw_id = storage.get(STORAGE_PREFIX, OCW_WRITE_ID);
		let id = raw_id
			.as_deref()
			.map(|id| u64::from_be_bytes(id.try_into().unwrap()))
			.unwrap_or_default();
		if !storage.compare_and_set(
			STORAGE_PREFIX,
			OCW_WRITE_ID,
			raw_id.as_deref(),
			&(id + 1).to_be_bytes(),
		) {
			continue;
		}
		storage.set(STORAGE_PREFIX, &msg_key(id), &payload);
		break;
	}
}
