use crate::{CycleStatus, ShardId, TaskCycle, TaskError, TaskId, TssPublicKey};
use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};

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
