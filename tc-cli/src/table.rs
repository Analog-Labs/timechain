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
	gateway: String,
	gateway_balance: String,
	admin: String,
	admin_balance: String,
	read_events: TaskId,
	sync_status: String,
	unassigned_tasks: u32,
}

impl IntoRow for Network {
	type Row = NetworkEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		let mut gateway = String::new();
		let mut gateway_balance = String::new();
		let mut admin = String::new();
		let mut admin_balance = String::new();
		let mut read_events = 0;
		let mut unassigned_tasks = 0;
		let sync_status = if let Some(info) = self.info.as_ref() {
			gateway = tc.format_address(Some(self.network), info.gateway)?;
			gateway_balance = tc.format_balance(Some(self.network), info.gateway_balance)?;
			admin = tc.format_address(Some(self.network), info.admin)?;
			admin_balance = tc.format_balance(Some(self.network), info.admin_balance)?;
			read_events = info.sync_status.task;
			unassigned_tasks = info.unassigned_tasks;
			format!("{} / {}", info.sync_status.sync, info.sync_status.block)
		} else {
			"no connector configured".to_string()
		};
		Ok(NetworkEntry {
			network: self.network,
			chain_name: self.chain_name,
			gateway,
			gateway_balance,
			admin,
			admin_balance,
			read_events,
			sync_status,
			unassigned_tasks,
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
	batch_register: String,
	batch_unregister: String,
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
			batch_register: self.batch_register.map(|b| b.to_string()).unwrap_or_default(),
			batch_unregister: self.batch_unregister.map(|b| b.to_string()).unwrap_or_default(),
		})
	}
}

#[derive(Serialize)]
pub struct MemberEntry {
	account: String,
	status: String,
}

impl IntoRow for Member {
	type Row = MemberEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(MemberEntry {
			account: self.account.to_string(),
			status: self.status.to_string(),
		})
	}
}

#[derive(Serialize)]
pub struct RouteEntry {
	network: NetworkId,
	gateway: String,
	relative_gas_price: String,
	gas_limit: u64,
	gmp_base_fee: u128,
}

impl IntoRow for Route {
	type Row = RouteEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		let (num, den) = self.relative_gas_price;
		Ok(RouteEntry {
			network: self.network_id,
			gateway: tc.format_address(Some(self.network_id), self.gateway)?,
			// relative_gas_price: format!("{}", num as f64 / den as f64),
			// FIX the relative gas price string table
			relative_gas_price: format!("{:?}", num / den),
			gas_limit: self.gas_limit,
			gmp_base_fee: self.gmp_base_fee,
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
}

impl IntoRow for Task {
	type Row = TaskEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
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
		})
	}
}

#[derive(Serialize)]
pub struct BatchEntry {
	batch: BatchId,
	task: TaskId,
	tx: String,
}

impl IntoRow for Batch {
	type Row = BatchEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(BatchEntry {
			batch: self.batch,
			task: self.task,
			tx: self.tx.map(hex::encode).unwrap_or_else(|| "no tx hash".into()),
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

#[derive(Serialize)]
pub struct LogEntry {
	log_timestamp: String,
	log_level: String,
	log_message: String,
	log_filename: String,
	log_line_number: u64,
	log_target: String,
	tc_account: Option<String>,
	tc_block: Option<BlockNumber>,
	tc_block_hash: Option<String>,
	chain_address: Option<String>,
	chain_block: Option<u64>,
	net_peer_id: Option<String>,
	net_message: Option<String>,
	net_from: Option<String>,
	net_to: Option<String>,
	tss_session: Option<TaskId>,
	tss_coordinator: Option<bool>,
	tss_session_id: Option<u64>,
	gmp_network_id: Option<NetworkId>,
	gmp_message_id: Option<String>,
	gmp_batch_id: Option<BatchId>,
	gmp_batch: Option<String>,
	gmp_task_id: Option<TaskId>,
	gmp_task: Option<String>,
	gmp_shard_id: Option<ShardId>,
	gmp_events: Option<String>,
}

impl IntoRow for Log {
	type Row = LogEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(LogEntry {
			log_timestamp: self.log_timestamp.context("no timestamp")?,
			log_level: self.log_level.context("no log level")?,
			log_message: self.log_message.context("no log message")?,
			log_filename: self.log_filename.context("no filename")?,
			log_line_number: self.log_line_number.context("no line number")?,
			log_target: self.log_target.context("no log target")?,
			tc_account: self.tc_account,
			tc_block: self.tc_block,
			tc_block_hash: self.tc_block_hash,
			chain_address: self.chain_address,
			chain_block: self.chain_block,
			net_peer_id: self.net_peer_id,
			net_message: self.net_message,
			net_from: self.net_from,
			net_to: self.net_to,
			tss_session: self.tss_session,
			tss_coordinator: self.tss_coordinator,
			tss_session_id: self.tss_session_id,
			gmp_network_id: self.gmp_network_id,
			gmp_message_id: self.gmp_message_id,
			gmp_batch_id: self.gmp_batch_id,
			gmp_batch: self.gmp_batch,
			gmp_task_id: self.gmp_task_id,
			gmp_task: self.gmp_task,
			gmp_shard_id: self.gmp_shard_id,
			gmp_events: self.gmp_events,
		})
	}
}

#[derive(Serialize)]
pub struct BenchmarkEntry {
	src: NetworkId,
	dest: NetworkId,
	cost: String,
	messages: String,
	latency: String,
	throughput: String,
}

impl IntoRow for BenchmarkStats {
	type Row = BenchmarkEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(BenchmarkEntry {
			src: self.src,
			dest: self.dest,
			cost: format!("{:.3}$", self.msg_cost),
			messages: format!("{}/{}/{}", self.num_received, self.num_sent, self.num_total),
			latency: format!("{:.3} blocks", self.latency),
			throughput: format!("{:.3} msgs/block", self.throughput),
		})
	}
}
