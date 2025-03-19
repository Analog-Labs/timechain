use anyhow::Result;
use e2e_tests::TestEnvBuilder;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};

#[tokio::test]
async fn smoke() -> Result<()> {
	let mut builder = TestEnvBuilder::new().await?;
	builder.add_evm(2, 1, 1).await?;
	builder.add_evm(3, 1, 1).await?;
	let tc = builder.build().await?;
	let testers = tc.setup_test().await?;

	// Run smoke test
	tc.exec_smoke(2, 3, &testers, vec![42]).await?;

	// Restart chronicles
	let mut restart = FuturesUnordered::new();
	for chronicle in tc.chronicle_containers(2)? {
		restart.push(
			async move {
				chronicle.stop().await?;
				chronicle.start().await?;
				Ok::<_, anyhow::Error>(())
			}
			.boxed(),
		);
	}
	for chronicle in tc.chronicle_containers(3)? {
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
	tc.exec_smoke(2, 3, &testers, vec![42]).await?;

	Ok(())
}
