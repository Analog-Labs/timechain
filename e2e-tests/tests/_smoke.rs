use anyhow::Result;
use e2e_tests::{Backend, TestEnv, Tester};

async fn test_smoke(tc: Tester) -> Result<()> {
	tc.exec_smoke(0, 1, vec![42]).await?;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn smoke() -> Result<()> {
	let tc = Tester::new(true).await?;
	test_smoke(tc).await
}

#[tokio::test]
async fn smoke_evm() -> Result<()> {
	let (_env, tc) = TestEnv::new(Backend::Evm, false).await?;
	test_smoke(tc).await
}

#[tokio::test]
async fn smoke_grpc() -> Result<()> {
	let (_env, tc) = TestEnv::new(Backend::Grpc, false).await?;
	test_smoke(tc).await
}

#[tokio::test]
async fn smoke_grpc_tss() -> Result<()> {
	let (_env, tc) = TestEnv::new(Backend::Grpc, true).await?;
	test_smoke(tc).await
}

#[tokio::test]
async fn smoke_evm_tss() -> Result<()> {
	let (_env, tc) = TestEnv::new(Backend::Evm, true).await?;
	test_smoke(tc).await
}
