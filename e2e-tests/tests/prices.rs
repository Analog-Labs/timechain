use anyhow::Result;
use e2e_tests::{TestEnv, Tester, TestingBackend};

async fn test_prices() -> Result<()> {
	let mut tc = Tester::new().await?;
	tc.fetch_token_prices().await?;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn prices() -> Result<()> {
	test_prices().await
}

#[tokio::test]
#[ignore]
async fn prices_grpc() -> Result<()> {
	let _env = TestEnv::new(TestingBackend::Grpc, false).await?;
	test_prices().await
}
