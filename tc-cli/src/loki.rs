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
	//pub limit: Option<u32>,
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
pub enum Query {
	Chronicle {
		#[arg(long)]
		task: Option<TaskId>,
		#[arg(long)]
		shard: Option<ShardId>,
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
	},
	Raw {
		query: String,
	},
}

impl std::fmt::Display for Query {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Chronicle {
				task,
				shard,
				account,
				target_address,
				peer_id,
				block,
				block_hash,
				target_block,
			} => {
				write!(f, r#"{{app="chronicle"}}"#)?;
				if let Some(task) = task {
					write!(f, " |= `task_id: {task}`")?;
				}
				if let Some(shard) = shard {
					write!(f, " |= `shard_id: {shard}`")?;
				}
				if let Some(account) = account {
					write!(f, " |= `timechain: {account}`")?;
				}
				if let Some(address) = target_address {
					write!(f, " |= `target: {address}`")?;
				}
				if let Some(peer_id) = peer_id {
					write!(f, " |= `peer_id: {peer_id}`")?;
				}
				if let Some(block) = block {
					write!(f, " |= `block: {block}`")?;
				}
				if let Some(block_hash) = block_hash {
					write!(f, " |= `block_hash: {block_hash}`")?;
				}
				if let Some(block) = target_block {
					write!(f, " |= `target_block_height: {block}`")?;
				}
				Ok(())
			},
			Self::Raw { query } => f.write_str(query),
		}
	}
}

#[derive(Debug)]
struct Log {
	timestamp: String,
	level: String,
	module: String,
	msg: String,
	location: String,
	data: HashMap<String, String>,
}

impl std::str::FromStr for Log {
	type Err = anyhow::Error;

	fn from_str(log: &str) -> Result<Self> {
		let mut data = HashMap::new();
		let (timestamp, rest) = log.trim().split_once(' ').context("no timestamp")?;
		let (level, rest) = rest.split_once(' ').context("no level")?;
		let (module, rest) = rest.split_once(": ").context("no module")?;
		// Work around when logging raw byte arrays
		let (part1, rest) = rest.split_once(']').unwrap_or(("", rest));
		let (part2, rest) = rest.split_once(',').context("no msg")?;
		let msg = if part1.is_empty() { part2.to_string() } else { format!("{part1}]{part2}") };
		let (sdata, rest) = rest.split_once("  at ").context("no data")?;
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
			module: module.trim().into(),
			msg: msg.trim().into(),
			location: location.trim().into(),
			data,
		};
		Ok(me)
	}
}

impl std::fmt::Display for Log {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		writeln!(
			f,
			"{} {} {}: {} at {}",
			&self.timestamp, &self.level, &self.module, &self.msg, &self.location
		)?;
		writeln!(f, "{:#?}", self.data)
	}
}

pub async fn logs(query: Query) -> Result<Vec<Log>> {
	let env = Loki::from_env()?;
	let client = reqwest::Client::new();
	let url: reqwest::Url = format!("{}/loki/api/v1/query_range", &env.loki_url).parse()?;
	let req = client
		.get(url)
		.basic_auth(env.loki_username, Some(env.loki_password))
		.query(&Request {
			query: query.to_string(),
			since: "30d".into(),
		})
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

	Ok(resp
		.data
		.result
		.into_iter()
		.flat_map(|v| v.values)
		.map(|(_, log)| log.parse().unwrap())
		.collect::<Vec<Log>>())
}
