use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::Database;

pub mod feed;

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
