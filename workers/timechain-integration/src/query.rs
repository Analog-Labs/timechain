pub struct CollectData;
pub mod collect_data {
	#![allow(dead_code)]
	pub const OPERATION_NAME: &str = "CollectData";
	pub const QUERY : & str = "mutation CollectData($collection: String!, $block: Int!, $cycle: Int!, $taskId: Int!, $data: [String!]!) {\n  collect(collection: $collection, block: $block, cycle: $cycle, taskId: $taskId, data: $data) {\n    status\n  }\n}" ;

	use serde::{Deserialize, Serialize};
	#[allow(dead_code)]
	type Boolean = bool;
	#[allow(dead_code)]
	type Float = f64;
	#[allow(dead_code)]
	type Int = i64;
	#[allow(dead_code)]
	type ID = String;
	#[derive(Serialize)]
	pub struct Variables {
		pub collection: String,
		pub block: Int,
		pub cycle: Int,
		#[serde(rename = "taskId")]
		pub task_id: Int,
		pub data: Vec<String>,
		pub task_counter: i64,
		pub tss: String,
		pub event_id: i64,
	}
	impl Variables {}
	#[derive(Deserialize)]
	pub struct ResponseData {
		pub collect: CollectDataCollect,
	}
	#[derive(Deserialize, Debug)]
	pub struct CollectDataCollect {
		pub status: String,
	}
}
impl graphql_client::GraphQLQuery for CollectData {
	type Variables = collect_data::Variables;
	type ResponseData = collect_data::ResponseData;
	fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
		graphql_client::QueryBody {
			variables,
			query: collect_data::QUERY,
			operation_name: collect_data::OPERATION_NAME,
		}
	}
}
