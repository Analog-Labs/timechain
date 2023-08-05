use crate::{ScheduleCycle, ScheduleStatus, ShardId, TaskId, TssPublicKey};

pub enum OcwPayload {
	SubmitTssPublicKey { shard_id: ShardId, public_key: TssPublicKey },
	SubmitResult { task_id: TaskId, cycle: ScheduleCycle, status: ScheduleStatus },
}

impl OcwPayload {
	pub fn shard_id(&self) -> ShardId {
		match self {
			Self::SubmitTssPublicKey { shard_id, .. } => *shard_id,
			Self::SubmitResult { status, .. } => status.shard_id,
		}
	}
}
