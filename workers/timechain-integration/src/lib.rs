use crate::query::{CollectData, ResponseData, Variables};
use anyhow::{Context, Result};
use graphql_client::{GraphQLQuery, Response};
use reqwest::header;
use time_primitives::FunctionResult;

mod query;
#[cfg(test)]
mod test;

const TW_LOG: &str = "timegraph";

pub async fn submit_to_timegraph(
	collection: String,
	task_id: u64,
	cycle: u64,
	target_block_number: u64,
	block_num: i64,
	tss: [u8; 64],
	result: FunctionResult,
) -> Result<()> {
	// Add data into collection (user must have Collector role)
	// @collection: collection hashId
	// @task_id: task associated with data
	// @task_counter: for repeated task it's incremented on every run
	// @block: target network block number
	// @cycle: time-chain block number
	// @tss: TSS signature
	// @data: data to add into collection

	let FunctionResult::EVMViewWithoutAbi { result } = result;
	let variables = Variables {
		collection,
		task_id: task_id as i64,
		task_counter: cycle as i64,
		block: target_block_number as i64,
		cycle: block_num,
		tss: hex::encode(&tss),
		data: result,
	};
	dotenv::dotenv().ok();
	let url =
		std::env::var("TIMEGRAPH_GRAPHQL_URL").context("Unable to get timegraph graphql url")?;
	let ssk = std::env::var("SSK").context("Unable to get timegraph ssk")?;

	// Build the GraphQL request
	let request = CollectData::build_query(variables);
	// Execute the GraphQL request
	let client = reqwest::Client::new();
	let response = client
		.post(url)
		.json(&request)
		.header(header::AUTHORIZATION, ssk)
		.send()
		.await
		.map_err(|e| anyhow::anyhow!("error post to timegraph {}", e))?;
	let json = response
		.json::<Response<ResponseData>>()
		.await
		.context("Failed to parse timegraph response")?;
	let data = json
		.data
		.context(format!("timegraph migrate collect status fail: No reponse {:?}", json.errors))?;
	log::info!(target: TW_LOG, "timegraph migrate collect status: {:?}", data.collect.status);
	Ok(())
}
