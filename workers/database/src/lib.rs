use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::Database;

pub mod feed;
pub mod fetch_event;

pub use sea_orm::DatabaseConnection;

pub async fn connect() -> Result<DatabaseConnection> {
	let url = std::env::var("DATABASE_URL")
		.map_err(|_| anyhow::anyhow!("Error the DATABASE_URL not set."))?;
	Ok(Database::connect(url).await?)
}

pub async fn write_feed(conn: &mut DatabaseConnection, mut record: feed::Model) -> Result<()> {
	let now = TimeDateTimeWithTimeZone::now_utc();
	record.timestamp = Some(TimeDateTime::new(now.date(), now.time()));
	let record: feed::ActiveModel = record.into();
	feed::Entity::insert(record).exec(conn).await?;
	Ok(())
}

pub async fn write_fetch_event(
	conn: &mut DatabaseConnection,
	record: fetch_event::Model,
) -> Result<()> {
	let record: fetch_event::ActiveModel = record.into();
	fetch_event::Entity::insert(record).exec(conn).await?;
	Ok(())
}
