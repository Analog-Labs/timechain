use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fetch_event")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub block_number: i64,
	pub cycle: Option<i64>,
	pub value: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::fetch_event::Entity")]
	FetchEvent,
}

impl Related<super::fetch_event::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::FetchEvent.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
