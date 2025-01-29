use crate::*;
use anyhow::Result;
use serde::Serialize;
use time_primitives::GatewayOp;

pub trait IntoRow {
	type Row: Serialize;

	fn into_row(self, tc: &Tc) -> Result<Self::Row>;
}

#[derive(Serialize)]
pub struct NetworkEntry {
	network: NetworkId,
	chain_name: String,
	chain_network: String,
	gateway: String,
	gateway_balance: String,
	admin: String,
	admin_balance: String,
	read_events: TaskId,
	sync_status: String,
}

impl IntoRow for Network {
	type Row = NetworkEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		let mut gateway = String::new();
		let mut gateway_balance = String::new();
		let mut admin = String::new();
		let mut admin_balance = String::new();
		let mut read_events = 0;
		let sync_status = if let Some(info) = self.info.as_ref() {
			gateway = tc.format_address(Some(self.network), info.gateway)?;
			gateway_balance = tc.format_balance(Some(self.network), info.gateway_balance)?;
			admin = tc.format_address(Some(self.network), info.admin)?;
			admin_balance = tc.format_balance(Some(self.network), info.admin_balance)?;
			read_events = info.sync_status.task;
			format!("{} / {}", info.sync_status.sync, info.sync_status.block)
		} else {
			"no connector configured".to_string()
		};
		Ok(NetworkEntry {
			network: self.network,
			chain_name: self.chain_name,
			chain_network: self.chain_network,
			gateway,
			gateway_balance,
			admin,
			admin_balance,
			read_events,
			sync_status,
		})
	}
}

#[derive(Serialize)]
pub struct ChronicleEntry {
	network: NetworkId,
	account: String,
	peer_id: String,
	status: String,
	balance: String,
	target_address: String,
	target_balance: String,
}

impl IntoRow for Chronicle {
	type Row = ChronicleEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(ChronicleEntry {
			network: self.network,
			account: tc.format_address(None, self.account.into())?,
			peer_id: self.peer_id,
			status: self.status.to_string(),
			balance: tc.format_balance(None, self.balance)?,
			target_address: tc.format_address(Some(self.network), self.target_address)?,
			target_balance: tc.format_balance(Some(self.network), self.target_balance)?,
		})
	}
}

#[derive(Serialize)]
pub struct ShardEntry {
	shard: ShardId,
	network: NetworkId,
	status: String,
	key: String,
	registered: String,
	size: u16,
	threshold: u16,
	assigned: usize,
}

impl IntoRow for Shard {
	type Row = ShardEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(ShardEntry {
			shard: self.shard,
			network: self.network,
			status: self.status.to_string(),
			key: self.key.map(hex::encode).unwrap_or_default(),
			registered: self.registered.to_string(),
			size: self.size,
			threshold: self.threshold,
			assigned: self.assigned,
		})
	}
}

#[derive(Serialize)]
pub struct MemberEntry {
	account: String,
	status: String,
	staker: String,
	stake: String,
}

impl IntoRow for Member {
	type Row = MemberEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(MemberEntry {
			account: self.account.to_string(),
			status: self.status.to_string(),
			staker: self
				.staker
				.map(|staker| tc.format_address(None, staker.into()))
				.transpose()?
				.unwrap_or_default(),
			stake: tc.format_balance(None, self.stake)?,
		})
	}
}

#[derive(Serialize)]
pub struct RouteEntry {
	network: NetworkId,
	gateway: String,
	relative_gas_price: String,
	gas_limit: u64,
	base_fee: u128,
}

impl IntoRow for Route {
	type Row = RouteEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		let (num, den) = self.relative_gas_price;
		Ok(RouteEntry {
			network: self.network_id,
			gateway: tc.format_address(Some(self.network_id), self.gateway)?,
			relative_gas_price: format!("{}", num as f64 / den as f64),
			gas_limit: self.gas_limit,
			base_fee: self.base_fee,
		})
	}
}

#[derive(Serialize)]
pub struct EventEntry {
	event: String,
}

impl IntoRow for GmpEvent {
	type Row = EventEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(EventEntry { event: self.to_string() })
	}
}

#[derive(Serialize)]
pub struct MessageEntry {
	id: String,
	source_network: NetworkId,
	source_address: String,
	dest_network: NetworkId,
	dest_address: String,
}

impl IntoRow for GmpMessage {
	type Row = MessageEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(MessageEntry {
			id: self.to_string(),
			source_network: self.src_network,
			source_address: tc.format_address(Some(self.src_network), self.src)?,
			dest_network: self.dest_network,
			dest_address: tc.format_address(Some(self.dest_network), self.dest)?,
		})
	}
}

#[derive(Serialize)]
pub struct TaskEntry {
	task: TaskId,
	network: NetworkId,
	descriptor: String,
	output: String,
	shard: String,
	submitter: String,
}

impl IntoRow for Task {
	type Row = TaskEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(TaskEntry {
			task: self.task,
			network: self.network,
			descriptor: self.descriptor.to_string(),
			output: match self.output {
				Some(Ok(())) => "complete".to_string(),
				Some(Err(err)) => err,
				None => "in progress".to_string(),
			},
			shard: match self.shard {
				Some(shard) => shard.to_string(),
				None => "unassigned".to_string(),
			},
			submitter: match self.submitter {
				Some(submitter) => tc.format_address(None, submitter.into_account().into())?,
				None => "".to_string(),
			},
		})
	}
}

#[derive(Serialize)]
pub struct BatchEntry {
	batch: BatchId,
	task: TaskId,
}

impl IntoRow for Batch {
	type Row = BatchEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(BatchEntry {
			batch: self.batch,
			task: self.task,
		})
	}
}

#[derive(Serialize)]
pub struct BatchOpEntry {
	op: String,
}

impl IntoRow for GatewayOp {
	type Row = BatchOpEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(BatchOpEntry { op: self.to_string() })
	}
}

#[derive(Serialize)]
pub struct MessageInfoEntry {
	message: String,
	recv: String,
	batch: String,
	exec: String,
}

impl IntoRow for Message {
	type Row = MessageInfoEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(MessageInfoEntry {
			message: hex::encode(self.message),
			recv: self.recv.map(|t| t.to_string()).unwrap_or_default(),
			batch: self.batch.map(|b| b.to_string()).unwrap_or_default(),
			exec: self.exec.map(|t| t.to_string()).unwrap_or_default(),
		})
	}
}

#[derive(Serialize)]
pub struct MessageTraceEntry {
	message: String,
	src_sync: String,
	dest_sync: String,
	recv: String,
	submit: String,
	exec: String,
}

impl IntoRow for MessageTrace {
	type Row = MessageTraceEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		fn task_to_string(task: Task) -> String {
			let status = if let Some(output) = task.output {
				match output {
					Ok(()) => "complete".to_string(),
					Err(err) => format!("failed '{err}'"),
				}
			} else if let Some(shard) = task.shard {
				format!("assigned to {}", shard)
			} else {
				"unassigned".to_string()
			};
			format!("{} {}", task.task, status)
		}
		Ok(MessageTraceEntry {
			message: hex::encode(self.message),
			src_sync: format!("{} / {}", self.src.sync, self.src.block),
			dest_sync: if let Some(sync) = self.dest {
				format!("{} / {}", sync.sync, sync.block)
			} else {
				"- / -".into()
			},
			recv: self.recv.map(task_to_string).unwrap_or_default(),
			submit: self.submit.map(task_to_string).unwrap_or_default(),
			exec: self.exec.map(task_to_string).unwrap_or_default(),
		})
	}
}
