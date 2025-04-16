use alloy_primitives::Address;
use anyhow::Result;
use e2e_tests::{Container, TestEnv, TestEnvBuilder, Tester};
use scale_value::Composite;
use subxt::config::substrate::{AccountId32, BlakeTwo256};
use subxt::config::Hasher;
use subxt::dynamic::Value;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;
use testcontainers::{core::IntoContainerPort, ContainerRequest, GenericImage, ImageExt};
use time_primitives::Address32;

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

	async fn fund(self, container: &Container, address: Address32, amount: u128) -> Result<()> {
		let url = format!(
			"ws://{}:{}",
			container.get_host().await?,
			container.get_host_port_ipv4(8545.tcp()).await?
		);

		// convert address
		let dest = {
			let address = Address::from_word(address.into());
			let mut data = [0u8; 24];
			data[0..4].copy_from_slice(b"evm:");
			data[4..24].copy_from_slice(&address[..]);
			let hash = BlakeTwo256::hash(&data);
			AccountId32::from(Into::<[u8; 32]>::into(hash))
		};

		// Build the transfer transaction
		let payload = subxt::tx::dynamic(
			"Balances",
			"transfer_allow_death",
			vec![
				Value::variant("Id", Composite::Unnamed(vec![Value::from_bytes(dest)])),
				amount.into(),
			],
		);
		OnlineClient::<PolkadotConfig>::from_insecure_url(url)
			.await?
			.tx()
			.sign_and_submit_then_watch_default(&payload, &dev::alice())
			.await?
			.wait_for_finalized_success()
			.await?;
		Ok(())
	}

	async fn setup(self) -> Result<(TestEnv, Tester)> {
		let mut builder = TestEnvBuilder::new(None).await?;
		builder.add_evm_custom(0, 1, 1, self.into_image()).await?;
		builder.add_evm_custom(1, 1, 1, self.into_image()).await?;
		let env = builder.build().await?;
		let mut tc = Tester::new().await?;
		self.fund(env.chain_container(0)?, tc.address(Some(0))?, tc.parse_balance(Some(0), "11.")?)
			.await?;
		self.fund(env.chain_container(1)?, tc.address(Some(1))?, tc.parse_balance(Some(1), "11.")?)
			.await?;
		tc.setup_test().await?;
		Ok((env, tc))
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
	let (_env, tc) = Chain::Astar.setup().await?;
	test_inner_revert(tc).await
}
