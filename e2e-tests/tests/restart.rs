use anyhow::Result;
use e2e_tests::{Backend, TestEnv, Tester};
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};

async fn chronicle_restart(env: &TestEnv, tc: Tester) -> Result<()> {
	// Restart chronicles
	let mut restart = FuturesUnordered::new();
	for chronicle in env.chronicle_containers(0)? {
		restart.push(
			async move {
				chronicle.stop().await?;
				chronicle.start().await?;
				Ok::<_, anyhow::Error>(())
			}
			.boxed(),
		);
	}
	for chronicle in env.chronicle_containers(1)? {
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

async fn chain_restart(env: &TestEnv, tc: Tester) -> Result<()> {
	// Restart chains
	let mut restart = FuturesUnordered::new();
	let chain = env.chain_container(0)?;
	restart.push(
		async move {
			chain.stop().await?;
			chain.start().await?;
			Ok::<_, anyhow::Error>(())
		}
		.boxed(),
	);
	let chain = env.chain_container(1)?;
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

async fn validator_restart(env: &TestEnv, tc: Tester) -> Result<()> {
	// Restart validator
	env.validator_container().stop().await?;
	env.validator_container().start().await?;

	// Re-run smoke test: should still work
	tc.smoke_test(vec![42]).await?;

	Ok(())
}

#[tokio::test]
async fn chronicle_restart_evm_tss() -> Result<()> {
	let (env, tc) = TestEnv::new(Backend::Evm, true).await?;
	chronicle_restart(&env, tc).await
}

#[tokio::test]
async fn chain_restart_evm() -> Result<()> {
	let (env, tc) = TestEnv::new(Backend::Evm, false).await?;
	chain_restart(&env, tc).await
}

#[tokio::test]
async fn validator_restart_grpc() -> Result<()> {
	let (env, tc) = TestEnv::new(Backend::Grpc, false).await?;
	validator_restart(&env, tc).await
}
