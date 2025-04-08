use anyhow::Result;
use e2e_tests::{Backend, TestEnv, Tester};

async fn test_smoke() -> Result<()> {
	let tc = Tester::new().await?;
	tc.smoke_test(vec![42]).await?;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn smoke() -> Result<()> {
	test_smoke().await
}

#[tokio::test]
async fn smoke_evm() -> Result<()> {
	let _env = TestEnv::new(Backend::Evm, false).await?;
	test_smoke().await
}

#[tokio::test]
async fn smoke_grpc() -> Result<()> {
	let _env = TestEnv::new(Backend::Grpc, false).await?;
	test_smoke().await
}

#[tokio::test]
async fn smoke_grpc_tss() -> Result<()> {
	let _env = TestEnv::new(Backend::Grpc, true).await?;
	test_smoke().await
}

#[tokio::test]
async fn smoke_evm_tss() -> Result<()> {
	let _env = TestEnv::new(Backend::Evm, true).await?;
	test_smoke().await
}
