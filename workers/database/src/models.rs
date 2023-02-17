use diesel::prelude::*;

#[derive(Queryable, Debug)]
pub struct OnChainData {
	pub data_id: i32,
	pub task_id: i32,
	pub block_number: i32,
	pub time_stamp: String,
	pub on_chain_data: String,
}
