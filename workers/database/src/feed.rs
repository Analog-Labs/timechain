use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "_feeds_")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i64,
	pub hash: String,
	pub task: Vec<u8>,
	pub validity: i64,
	pub timestamp: Option<TimeDateTime>,
	pub cycle: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::feed::Entity")]
	Feed,
}

impl Related<super::feed::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Feed.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
