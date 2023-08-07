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
	target_block_number: u64,
	result: &FunctionResult,
	collection: String,
) -> Result<()> {
	// Add data into collection (user must have Collector role)
	// @collection: collection hashId
	// @cycle: time-chain block number
	// @block: target network block number
	// @task_id: task associated with data
	// @task_counter: for repeated task it's incremented on every run
	// @tss: TSS signature
	// @data: data to add into collection

	let FunctionResult::EVMViewWithoutAbi { result } = result;
	let variables = Variables {
		collection,
		block: target_block_number as i64,
		// unused field
		task_id: 0,
		// unused field
		cycle: 0,
		data: result.clone(),
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
		.context("error in post request to timegraph")?;
	let data = response
		.json::<Response<ResponseData>>()
		.await
		.context("Failed to parse timegraph response")?
		.data
		.context("timegraph migrate collect status fail: No reponse")?;
	log::info!(target: TW_LOG, "timegraph migrate collect status: {:?}", data.collect.status);
	Ok(())
}
