use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "on_chain_data")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub data_id: i32,
	pub task_id: i32,
	pub block_number: i32,
	pub time_stamp: TimeDateTime,
	pub on_chain_data: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::on_chain_data::Entity")]
	OnChainData,
}

impl Related<super::on_chain_data::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::OnChainData.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
