use sea_orm_migration::prelude::*;

#[async_std::main]
async fn main() {
	cli::run_cli(time_db_migration::Migrator).await;
}
