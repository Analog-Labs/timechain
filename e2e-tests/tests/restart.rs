use anyhow::Result;
use e2e_tests::{Backend, TestEnvBuilder};
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};

async fn chronicle_restart(backend: Backend, shard_size: u16) -> Result<()> {
	let tc = TestEnvBuilder::setup(backend, shard_size, shard_size).await?;

	// Restart chronicles
	let mut restart = FuturesUnordered::new();
	for chronicle in tc.chronicle_containers(0)? {
		restart.push(
			async move {
				chronicle.stop().await?;
				chronicle.start().await?;
				Ok::<_, anyhow::Error>(())
			}
			.boxed(),
		);
	}
	for chronicle in tc.chronicle_containers(1)? {
		restart.push(
			async move {
				chronicle.stop().await?;
				chronicle.start().await?;
				Ok::<_, anyhow::Error>(())
			}
			.boxed(),
		);
	}
	while let Some(result) = restart.next().await {
		result?;
	}

	// Re-run smoke test: should still work
	tc.smoke_test(vec![42]).await?;

	Ok(())
}

async fn chain_restart(backend: Backend, shard_size: u16) -> Result<()> {
	let tc = TestEnvBuilder::setup(backend, shard_size, shard_size).await?;

	// Restart chains
	let mut restart = FuturesUnordered::new();
	let chain = tc.chain_container(0)?;
	restart.push(
		async move {
			chain.stop().await?;
			chain.start().await?;
			Ok::<_, anyhow::Error>(())
		}
		.boxed(),
	);
	let chain = tc.chain_container(1)?;
	restart.push(
		async move {
			chain.stop().await?;
			chain.start().await?;
			Ok::<_, anyhow::Error>(())
		}
		.boxed(),
	);
	while let Some(result) = restart.next().await {
		result?;
	}

	// Re-run smoke test: should still work
	tc.smoke_test(vec![42]).await?;

	Ok(())
}

async fn validator_restart(backend: Backend, shard_size: u16) -> Result<()> {
	let tc = TestEnvBuilder::setup(backend, shard_size, shard_size).await?;

	// Restart validator
	tc.validator_container().stop().await?;
	tc.validator_container().start().await?;

	// Re-run smoke test: should still work
	tc.smoke_test(vec![42]).await?;

	Ok(())
}

#[tokio::test]
#[ignore]
async fn chronicle_restart_evm_tss() -> Result<()> {
	chronicle_restart(Backend::Evm, 3).await
}

#[tokio::test]
#[ignore]
async fn chain_restart_evm() -> Result<()> {
	// TODO: requires dumping/loading anvil state
	chain_restart(Backend::Evm, 1).await
}

#[tokio::test]
#[ignore]
async fn validator_restart_grpc() -> Result<()> {
	// TODO: fails to reconnect
	validator_restart(Backend::Grpc, 1).await
}
