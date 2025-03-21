use anyhow::Result;
use e2e_tests::{Backend, TestEnvBuilder};

async fn smoke(backend: Backend, shard_size: u16) -> Result<()> {
	let tc = TestEnvBuilder::setup(backend, shard_size, shard_size).await?;
	tc.smoke_test(vec![42]).await?;
	Ok(())
}

#[tokio::test]
async fn smoke_evm() -> Result<()> {
	smoke(Backend::Evm, 1).await
}

#[tokio::test]
async fn smoke_grpc() -> Result<()> {
	smoke(Backend::Grpc, 1).await
}

#[tokio::test]
#[ignore]
async fn smoke_grpc_tss() -> Result<()> {
	smoke(Backend::Grpc, 3).await
}
