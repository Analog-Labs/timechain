use anyhow::Result;
use e2e_tests::{TestEnvBuilder, Tester};
use testcontainers::{core::IntoContainerPort, ContainerRequest, GenericImage, ImageExt};

#[derive(Clone, Copy, Debug)]
enum Chain {
	Astar,
}

impl Chain {
	fn into_image(self) -> ContainerRequest<GenericImage> {
		GenericImage::new("staketechnologies/astar-collator", "v5.28.0-rerun")
			.with_exposed_port(8545.tcp())
			.with_cmd([
				"astar-collator",
				"--chain=dev",
				"--rpc-cors=all",
				"--rpc-external",
				"--rpc-port=8545",
				"--enable-evm-rpc",
				"--alice",
				"--tmp",
			])
	}
}

async fn test_inner_revert(tc: Tester) -> Result<()> {
	tc.exec_smoke(0, 1, vec![42], Some(0)).await?;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn inner_revert() -> Result<()> {
	let tc = Tester::new().await?;
	test_inner_revert(tc).await
}

#[tokio::test]
async fn inner_revert_astar() -> Result<()> {
	let mut builder = TestEnvBuilder::new(None).await?;
	builder.add_evm_custom(0, 1, 1, Chain::Astar.into_image()).await?;
	builder.add_evm_custom(1, 1, 1, Chain::Astar.into_image()).await?;
	let _env = builder.build().await?;
	let tc = Tester::new().await?;
	test_inner_revert(tc).await
}
