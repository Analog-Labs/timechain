use anyhow::Result;
use e2e_tests::{Backend, TestEnv, Tester};

#[tokio::test]
#[ignore]
async fn prices() -> Result<()> {
	let mut tc = Tester::new().await?;
	tc.fetch_token_prices().await?;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn prices_grpc() -> Result<()> {
	let _env = TestEnv::new(Backend::Grpc, false).await?;
	prices()?;
	Ok(())
}
