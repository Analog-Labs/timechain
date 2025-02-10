use crate::env::Loki;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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
		task: Option<TaskId>,
		shard: Option<ShardId>,
		account: Option<String>,
		target_address: Option<String>,
		peer_id: Option<String>,
		block: Option<BlockNumber>,
		block_hash: Option<String>,
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

pub async fn logs(query: Query) -> Result<Vec<String>> {
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
	log::info!("GET {}", req.url());
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
		.flat_map(|(_, log)| log.split("   ").map(|s| s.to_string()).collect::<Vec<_>>())
		.collect())
}
