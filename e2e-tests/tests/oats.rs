use anyhow::Result;
use e2e_tests::{Backend, TestEnv};

// #[tokio::test]
// async fn forever() -> Result<()> {
// 	let (_env, _tc) = TestEnv::new(Backend::Evm, false).await?;
// 	loop {}
// }


#[tokio::test]
async fn forever() -> Result<()> {
	let (_env, _tc) = TestEnv::new(Backend::Evm, false).await?;
    tracing::info!("Test env ready. Keeping live indefinitely...");
	loop {}
}
