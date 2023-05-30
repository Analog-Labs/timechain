use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue, Database};
use time_db_migration::MigratorTrait;

mod entities;

pub use crate::entities::*;
pub use sea_orm::DatabaseConnection;

pub async fn connect() -> Result<DatabaseConnection> {
	let url = std::env::var("DATABASE_URL")
		.map_err(|_| anyhow::anyhow!("Error the DATABASE_URL not set."))?;
	let db = Database::connect(url).await?;
	time_db_migration::Migrator::up(&db, None).await?;
	Ok(db)
}

pub async fn write_feed(conn: &mut DatabaseConnection, record: feed::Model) -> Result<()> {
	let mut record: feed::ActiveModel = record.into();
	let now = TimeDateTimeWithTimeZone::now_utc();
	record.id = ActiveValue::NotSet;
	record.timestamp = ActiveValue::Set(Some(TimeDateTime::new(now.date(), now.time())));
	feed::Entity::insert(record).exec(conn).await?;
	Ok(())
}

pub async fn write_fetch_event(
	conn: &mut DatabaseConnection,
	record: fetch_event::Model,
) -> Result<()> {
	let mut record: fetch_event::ActiveModel = record.into();
	record.id = ActiveValue::NotSet;
	fetch_event::Entity::insert(record).exec(conn).await?;
	Ok(())
}
