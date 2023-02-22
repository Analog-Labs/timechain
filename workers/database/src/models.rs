use diesel::prelude::*;
use std::time::SystemTime;

#[derive(Queryable, PartialEq, Debug)]
pub struct OnChainData {
	pub data_id: i32,
	pub task_id: i32,
	pub block_number: i32,
	pub time_stamp: SystemTime,
	pub on_chain_data: String,
}
