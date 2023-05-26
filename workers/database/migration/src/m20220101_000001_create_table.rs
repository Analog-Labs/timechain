use sea_orm_migration::prelude::*;

#[derive(Iden)]
enum Feed {
	Table,
	Id,
	TaskId,
	Hash,
	Task,
	Validity,
	Timestamp,
	Cycle,
}

#[derive(Iden)]
enum FetchEvent {
	Table,
	Id,
	BlockNumber,
	Cycle,
	Value,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(Feed::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(Feed::Id)
							.integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(ColumnDef::new(Feed::TaskId).big_integer().not_null())
					.col(ColumnDef::new(Feed::Hash).string().not_null())
					.col(ColumnDef::new(Feed::Task).binary().not_null())
					.col(ColumnDef::new(Feed::Validity).integer().not_null())
					.col(ColumnDef::new(Feed::Timestamp).timestamp())
					.col(ColumnDef::new(Feed::Cycle).integer())
					.to_owned(),
			)
			.await?;
		manager
			.create_table(
				Table::create()
					.table(FetchEvent::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(FetchEvent::Id)
							.integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(ColumnDef::new(FetchEvent::BlockNumber).integer().not_null())
					.col(ColumnDef::new(FetchEvent::Cycle).integer())
					.col(ColumnDef::new(FetchEvent::Value).string().not_null())
					.to_owned(),
			)
			.await?;
		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Feed::Table).to_owned()).await?;
		manager.drop_table(Table::drop().table(FetchEvent::Table).to_owned()).await?;
		Ok(())
	}
}
