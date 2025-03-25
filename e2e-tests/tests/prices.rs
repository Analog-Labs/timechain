use anyhow::Result;
use e2e_tests::{Backend, TestEnvBuilder};

async fn prices(backend: Backend, shard_size: u16) -> Result<()> {
	let mut tc = TestEnvBuilder::setup(backend, shard_size, shard_size).await?;
	tc.fetch_token_prices().await?;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn prices_grpc() -> Result<()> {
	prices(Backend::Grpc, 1).await
}
