use anyhow::Result;
use e2e_tests::TestEnvBuilder;

#[tokio::test]
async fn smoke() -> Result<()> {
	let mut builder = TestEnvBuilder::new();
	builder.add_evm(2, 1, 1);
	builder.add_evm(3, 1, 1);
	let tc = builder.build().await?;
	let testers = tc.setup_test().await?;

	// Run smoke test
	tc.exec_smoke(2, 3, &testers, vec![42]).await?;

	// Restart chronicles
	tc.restart(&["chronicle-2-evm", "chronicle-3-evm"])?;

	// Re-run smoke test: should still work
	tc.exec_smoke(2, 3, &testers, vec![42]).await?;

	Ok(())
}
