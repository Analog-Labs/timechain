use crate::env::Loki;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time_primitives::{BlockNumber, ShardId, TaskId};

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

#[derive(Clone, Debug, clap::Parser)]
pub struct Filter {
	#[arg(long)]
	task_id: Option<TaskId>,
	#[arg(long)]
	shard_id: Option<ShardId>,
	#[arg(long)]
	task: Option<String>,
	#[arg(long)]
	account: Option<String>,
	#[arg(long)]
	target_address: Option<String>,
	#[arg(long)]
	peer_id: Option<String>,
	#[arg(long)]
	block: Option<BlockNumber>,
	#[arg(long)]
	block_hash: Option<String>,
	#[arg(long)]
	target_block: Option<u64>,
	#[arg(long)]
	from: Option<String>,
	#[arg(long)]
	to: Option<String>,
}

impl Filter {
	pub fn has_filter(&self) -> bool {
		self.task_id.is_some()
			|| self.shard_id.is_some()
			|| self.task.is_some()
			|| self.account.is_some()
			|| self.target_address.is_some()
			|| self.peer_id.is_some()
			|| self.block.is_some()
			|| self.block_hash.is_some()
			|| self.target_block.is_some()
			|| self.from.is_some()
			|| self.to.is_some()
	}
}

impl std::fmt::Display for Filter {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		if let Some(task) = self.task_id {
			write!(f, " |= `task_id: {task},`")?;
		}
		if let Some(shard) = self.shard_id {
			write!(f, " |= `shard_id: {shard},`")?;
		}
		if let Some(task) = self.task.as_ref() {
			write!(f, r#" |= `task: "{task}"`"#)?;
		}
		if let Some(account) = self.account.as_ref() {
			write!(f, r#" |= `timechain: "{account}"`"#)?;
		}
		if let Some(address) = self.target_address.as_ref() {
			write!(f, r#" |= `target: "{address}"`"#)?;
		}
		if let Some(peer_id) = self.peer_id.as_ref() {
			write!(f, r#" |= `peer_id: "{peer_id}"`"#)?;
		}
		if let Some(block) = self.block {
			write!(f, " |= `block: {block},`")?;
		}
		if let Some(block_hash) = self.block_hash.as_ref() {
			write!(f, r#" |= `block_hash: "{block_hash}"`"#)?;
		}
		if let Some(block) = self.target_block {
			write!(f, " |= `target_block_height: {block},`")?;
		}
		if let Some(from) = self.from.as_ref() {
			write!(f, r#" |= `from: "{from}"`"#)?;
		}
		if let Some(to) = self.to.as_ref() {
			write!(f, r#" |= `to: "{to}"`"#)?;
		}
		Ok(())
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub enum Query {
	App {
		name: String,
		#[command(flatten)]
		filter: Filter,
	},
	Container {
		name: String,
		#[command(flatten)]
		filter: Filter,
	},
	Raw {
		query: String,
		#[command(flatten)]
		filter: Filter,
	},
}

impl Query {
	pub fn filter(&self) -> &Filter {
		match self {
			Self::App { filter, .. } => filter,
			Self::Container { filter, .. } => filter,
			Self::Raw { filter, .. } => filter,
		}
	}
}

impl std::fmt::Display for Query {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let filter = match self {
			Self::App { name, filter } => {
				write!(f, r#"{{app="{name}"}}"#)?;
				filter
			},
			Self::Container { name, filter } => {
				write!(f, r#"{{container=~"{name}"}}"#)?;
				filter
			},
			Self::Raw { query, filter } => {
				f.write_str(query)?;
				filter
			},
		};
		write!(f, "{filter}")
	}
}

#[derive(Debug)]
pub struct Log {
	pub timestamp: String,
	pub level: String,
	pub msg: String,
	pub location: String,
	pub data: HashMap<String, String>,
}

impl std::str::FromStr for Log {
	type Err = anyhow::Error;

	fn from_str(log: &str) -> Result<Self> {
		let mut data = HashMap::new();
		let (timestamp, rest) = log.trim().split_once(' ').context("no timestamp")?;
		let (level, rest) = rest.trim().split_once(' ').context("no level")?;
		let (_module, rest) = rest.split_once(": ").context("no module")?;
		let (mrest, rest) = rest.split_once("  at ").context("no data")?;
		// Work around when logging raw byte arrays
		let (part1, mrest) = mrest.split_once(']').unwrap_or(("", mrest));
		let (part2, sdata) = mrest.split_once(',').unwrap_or((mrest, ""));
		let msg = if part1.is_empty() { part2.to_string() } else { format!("{part1}]{part2}") };
		for kv in sdata.split(',') {
			let kv = kv.trim();
			if kv.is_empty() {
				continue;
			}
			let (k, v) = kv.split_once(':').context("no kv")?;
			data.insert(k.trim().to_string(), v.trim().to_string());
		}
		let (location, rest) = rest.split_once("  ").unwrap_or((rest, ""));
		for span in rest.split("  in ") {
			let Some((_, sdata)) = span.split_once(" with ") else {
				continue;
			};
			for kv in sdata.split(',') {
				let kv = kv.trim();
				if kv.is_empty() {
					continue;
				}
				let (k, v) = kv.split_once(':').context("span no kv")?;
				data.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
			}
		}
		let me = Self {
			timestamp: timestamp.trim().into(),
			level: level.trim().into(),
			msg: msg.trim().into(),
			location: location.trim().into(),
			data,
		};
		Ok(me)
	}
}

pub async fn raw_logs(query: Query, since: String, limit: Option<u32>) -> Result<Vec<String>> {
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

pub fn structured_logs(logs: &[String]) -> Result<Vec<Log>> {
	let mut slogs = Vec::with_capacity(logs.len());
	for log in logs {
		slogs.push(log.parse().with_context(|| format!("line {log}"))?);
	}
	Ok(slogs)
}
