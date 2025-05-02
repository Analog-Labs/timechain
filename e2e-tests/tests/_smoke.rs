use anyhow::Result;
use e2e_tests::{TestEnv, Tester, TestingBackend};

async fn test_smoke(tc: Tester) -> Result<()> {
	tc.exec_smoke(0, 1, vec![42]).await?;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn smoke() -> Result<()> {
	let tc = Tester::new().await?;
	test_smoke(tc).await
}

#[tokio::test]
async fn smoke_evm() -> Result<()> {
	let (_env, tc) = TestEnv::new(TestingBackend::evm_local(), false).await?;
	test_smoke(tc).await
}

#[tokio::test]
async fn smoke_grpc() -> Result<()> {
	let (_env, tc) = TestEnv::new(TestingBackend::Grpc, false).await?;
	test_smoke(tc).await
}

#[tokio::test]
async fn smoke_grpc_tss() -> Result<()> {
	let (_env, tc) = TestEnv::new(TestingBackend::Grpc, true).await?;
	test_smoke(tc).await
}

#[tokio::test]
async fn smoke_evm_tss() -> Result<()> {
	let (_env, tc) = TestEnv::new(TestingBackend::evm_local(), true).await?;
	test_smoke(tc).await
}
