use crate::schema;
use chrono;
use diesel::prelude::*;
use schema::_feeds_;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::time::SystemTime;

#[derive(Queryable, PartialEq, Debug)]
pub struct OnChainData {
	pub data_id: i32,
	pub task_id: i32,
	pub block_number: i32,
	pub time_stamp: SystemTime,
	pub on_chain_data: String,
}

impl Serialize for OnChainData {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_struct("OnChainData", 5)?;
		state.serialize_field("data_id", &self.data_id)?;
		state.serialize_field("task_id", &self.task_id)?;
		state.serialize_field("block_number", &self.block_number)?;
		state.serialize_field("time_stamp", &self.time_stamp)?;
		state.serialize_field("on_chain_data", &self.on_chain_data)?;
		state.end()
	}
}

#[derive(Insertable, Debug)]
#[table_name = "_feeds_"]
pub struct Feeds {
	pub id: i64,
	pub hash: String,
	pub task: Vec<u8>,
	pub validity: i64,
	pub timestamp: Option<chrono::NaiveDateTime>,
	pub cycle: Option<i64>,
}
