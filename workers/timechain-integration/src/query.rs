use graphql_client::{GraphQLQuery, QueryBody};
use serde::{Deserialize, Serialize};

pub const OPERATION_NAME: &str = "CollectData";
pub const QUERY : & str = "mutation CollectData($collection: String!, $block: Int!, $cycle: Int!, $taskId: Int!, $data: [String!]!) {\n  collect(collection: $collection, block: $block, cycle: $cycle, taskId: $taskId, data: $data) {\n    status\n  }\n}" ;

type Int = i64;

#[derive(Serialize, Debug)]
pub struct Variables {
	pub collection: String,
	pub block: Int,
	pub cycle: Int,
	#[serde(rename = "taskId")]
	pub task_id: Int,
	pub data: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct ResponseData {
	pub collect: CollectDataCollect,
}

#[derive(Deserialize, Debug)]
pub struct CollectDataCollect {
	pub status: String,
}

pub struct CollectData;

impl GraphQLQuery for CollectData {
	type Variables = Variables;
	type ResponseData = ResponseData;
	fn build_query(variables: Self::Variables) -> QueryBody<Self::Variables> {
		QueryBody {
			variables,
			query: QUERY,
			operation_name: OPERATION_NAME,
		}
	}
}
