use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::Database;

pub mod feed;
pub mod on_chain_data;

pub use sea_orm::DatabaseConnection;

const DEFAULT_LIMIT: u64 = 1000;

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

pub async fn read_on_chain_data(
	conn: &mut DatabaseConnection,
	min_id: i32,
) -> Result<Vec<on_chain_data::Model>> {
	Ok(on_chain_data::Entity::find()
		.cursor_by(on_chain_data::Column::DataId)
		.after(min_id)
		.first(DEFAULT_LIMIT)
		.all(conn)
		.await?)
}
