use crate::env::Loki;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use time_primitives::{BatchId, BlockNumber, NetworkId, ShardId, TaskId};

//const DIRECTION_FORWARD: &'static str = "FORWARD";
//const DIRECTION_BACKWARD: &'static str = "BACKWARD";

#[derive(Serialize)]
struct Request {
	pub query: String,
	pub since: String,
	pub limit: Option<u32>,
	//pub direction: Option<&'static str>,
}

#[derive(Debug, Deserialize)]
struct Response {
	pub status: String,
	pub data: LogData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogData {
	pub result_type: String,
	pub result: Vec<StreamValue>,
}

#[derive(Debug, Deserialize)]
struct StreamValue {
	pub values: Vec<(String, String)>,
}

#[derive(Clone, Debug, clap::Parser, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Log {
	#[arg(long)]
	#[serde(rename = "timestamp")]
	pub log_timestamp: Option<String>,
	#[arg(long)]
	#[serde(rename = "level")]
	pub log_level: Option<String>,
	#[arg(long)]
	#[serde(rename = "message")]
	pub log_message: Option<String>,
	#[arg(long)]
	#[serde(rename = "target")]
	pub log_target: Option<String>,
	#[arg(long)]
	#[serde(rename = "filename")]
	pub log_filename: Option<String>,
	#[arg(long)]
	#[serde(rename = "line_number")]
	pub log_line_number: Option<u64>,
	#[serde(rename = "name")]
	_log_name: Option<String>,
	#[arg(long)]
	pub tc_account: Option<String>,
	#[arg(long)]
	pub tc_block: Option<BlockNumber>,
	#[arg(long)]
	pub tc_block_hash: Option<String>,
	#[arg(long)]
	pub chain_address: Option<String>,
	#[arg(long)]
	pub chain_block: Option<u64>,
	#[arg(long)]
	pub net_peer_id: Option<String>,
	#[arg(long)]
	pub net_message: Option<String>,
	#[arg(long)]
	pub net_from: Option<String>,
	#[arg(long)]
	pub net_to: Option<String>,
	#[arg(long)]
	pub tss_session: Option<TaskId>,
	#[arg(long)]
	pub tss_coordinator: Option<bool>,
	#[arg(long)]
	pub tss_session_id: Option<u64>,
	#[arg(long)]
	pub gmp_network_id: Option<NetworkId>,
	#[arg(long)]
	pub gmp_message_id: Option<String>,
	#[arg(long)]
	pub gmp_batch_id: Option<BatchId>,
	#[arg(long)]
	pub gmp_batch: Option<String>,
	#[arg(long)]
	pub gmp_task_id: Option<TaskId>,
	#[arg(long)]
	pub gmp_task: Option<String>,
	#[arg(long)]
	pub gmp_shard_id: Option<ShardId>,
	#[arg(long)]
	pub gmp_events: Option<String>,
}

impl Log {
	pub fn has_fields(&self) -> bool {
		self.log_timestamp.is_some()
			|| self.log_level.is_some()
			|| self.log_message.is_some()
			|| self.log_target.is_some()
			|| self.log_filename.is_some()
			|| self.log_line_number.is_some()
			|| self.tc_account.is_some()
			|| self.tc_block.is_some()
			|| self.tc_block_hash.is_some()
			|| self.chain_address.is_some()
			|| self.chain_block.is_some()
			|| self.net_peer_id.is_some()
			|| self.net_message.is_some()
			|| self.net_from.is_some()
			|| self.net_to.is_some()
			|| self.tss_session.is_some()
			|| self.tss_coordinator.is_some()
			|| self.tss_session_id.is_some()
			|| self.gmp_network_id.is_some()
			|| self.gmp_message_id.is_some()
			|| self.gmp_batch_id.is_some()
			|| self.gmp_batch.is_some()
			|| self.gmp_task_id.is_some()
			|| self.gmp_task.is_some()
			|| self.gmp_shard_id.is_some()
			|| self.gmp_events.is_some()
	}

	pub fn matches(&self, other: &Log) -> bool {
		(self.log_timestamp.is_none() || self.log_timestamp == other.log_timestamp)
			&& (self.log_level.is_none() || self.log_level == other.log_level)
			&& (self.log_message.is_none() || self.log_message == other.log_message)
			&& (self.log_target.is_none() || self.log_target == other.log_target)
			&& (self.log_filename.is_none() || self.log_filename == other.log_filename)
			&& (self.log_line_number.is_none() || self.log_line_number == other.log_line_number)
			&& (self.tc_account.is_none() || self.tc_account == other.tc_account)
			&& (self.tc_block.is_none() || self.tc_block == other.tc_block)
			&& (self.tc_block_hash.is_none() || self.tc_block_hash == other.tc_block_hash)
			&& (self.chain_address.is_none() || self.chain_address == other.chain_address)
			&& (self.chain_block.is_none() || self.chain_block == other.chain_block)
			&& (self.net_peer_id.is_none() || self.net_peer_id == other.net_peer_id)
			&& (self.net_message.is_none() || self.net_message == other.net_message)
			&& (self.net_from.is_none() || self.net_from == other.net_from)
			&& (self.net_to.is_none() || self.net_to == other.net_to)
			&& (self.tss_session.is_none() || self.tss_session == other.tss_session)
			&& (self.tss_coordinator.is_none() || self.tss_coordinator == other.tss_coordinator)
			&& (self.tss_session_id.is_none() || self.tss_session_id == other.tss_session_id)
			&& (self.gmp_network_id.is_none() || self.gmp_network_id == other.gmp_network_id)
			&& (self.gmp_message_id.is_none() || self.gmp_message_id == other.gmp_message_id)
			&& (self.gmp_batch_id.is_none() || self.gmp_batch_id == other.gmp_batch_id)
			&& (self.gmp_batch.is_none() || self.gmp_batch == other.gmp_batch)
			&& (self.gmp_task_id.is_none() || self.gmp_task_id == other.gmp_task_id)
			&& (self.gmp_task.is_none() || self.gmp_task == other.gmp_task)
			&& (self.gmp_shard_id.is_none() || self.gmp_shard_id == other.gmp_shard_id)
			&& (self.gmp_events.is_none() || self.gmp_events == other.gmp_events)
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub enum Query {
	App {
		name: String,
		#[command(flatten)]
		filter: Log,
	},
	Container {
		name: String,
		#[command(flatten)]
		filter: Log,
	},
	Raw {
		query: String,
		#[command(flatten)]
		filter: Log,
	},
}

impl Query {
	pub fn filter(&self) -> &Log {
		match self {
			Self::App { filter, .. } => filter,
			Self::Container { filter, .. } => filter,
			Self::Raw { filter, .. } => filter,
		}
	}
}

impl std::fmt::Display for Query {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::App { name, .. } => {
				write!(f, r#"{{app="{name}"}}"#)
			},
			Self::Container { name, .. } => {
				write!(f, r#"{{container=~"{name}"}}"#)
			},
			Self::Raw { query, .. } => f.write_str(query),
		}
	}
}

pub async fn raw_logs(query: &Query, since: String, limit: Option<u32>) -> Result<Vec<String>> {
	let query = query.to_string();
	log::info!("{query}");
	let env = Loki::from_env()?;
	let client = reqwest::Client::new();
	let url: reqwest::Url = format!("{}/loki/api/v1/query_range", &env.loki_url).parse()?;
	let req = client
		.get(url)
		.basic_auth(env.loki_username, Some(env.loki_password))
		.query(&Request { query, since, limit })
		.build()
		.context("invalid request")?;
	log::debug!("GET {}", req.url());
	let resp = client.execute(req).await?;
	let status = resp.status();
	if status != 200 {
		let err = resp.text().await?;
		anyhow::bail!("{}: {err}", status);
	}
	let resp: Response = resp.json().await?;
	anyhow::ensure!(resp.status == "success", "unexpected status");
	anyhow::ensure!(resp.data.result_type == "streams", "unexpected result type");
	let logs = resp
		.data
		.result
		.into_iter()
		.flat_map(|v| v.values)
		.map(|(_, log)| log)
		.collect::<Vec<String>>();
	Ok(logs)
}

pub fn structured_logs(filter: &Log, logs: &[String]) -> Result<Vec<Log>> {
	let mut slogs = Vec::with_capacity(logs.len());
	for log in logs {
		// allow duplicate keys
		let slog: serde_json::Value = serde_json::from_str(log.as_str())?;
		let slog: Log = serde_json::from_value(slog)
			.map_err(|err| anyhow::anyhow!("failed to parse log: {err:?} {}", log.as_str()))?;
		if filter.matches(&slog) {
			slogs.push(slog);
		}
	}
	Ok(slogs)
}
