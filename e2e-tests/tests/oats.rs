use alloy::{network::EthereumWallet, providers::ProviderBuilder, signers::local::PrivateKeySigner};
use anyhow::Result;
use e2e_tests::{Backend, TestEnv};
use alloy::sol;

// Anvil's default account(1)
const BOB_KEY: &str = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(clippy::too_many_arguments)]
    #[allow(missing_docs)]
    #[sol(rpc)]
    OmniToken,
    "contracts/OmniToken.json"
);


#[tokio::test]
async fn oats_evm() -> Result<()> {
	let (env, tc) = TestEnv::new(Backend::Evm, false).await?;
	let block = tc.latest_block().await?.0;

	//    let mut chains = HashMap::<NetworkId, (RootProvider, Address32)>::new();

	let mut chains = vec![];

	for nw in tc.networks(block).await? {
		let gw = nw.info.unwrap().gateway;
		let nw_id = nw.network;
		let c = env.chain_container(nw_id).unwrap();

		let port = c.get_host_port_ipv4(8545).await.unwrap();
		let url = format!("http://localhost:{port}");
        let signer: PrivateKeySigner = BOB_KEY.parse()?;
        let wallet = EthereumWallet::from(signer);
		let rpc = ProviderBuilder::new()
            .wallet(wallet)
            .connect(url.as_str()).await?;

		chains.push((nw_id, rpc, gw));
	}

    // Deploy and setup token on every chain
    for (nw, rpc, gw) in chains {

    }

	Ok(())
}

#[tokio::test]
#[ignore]
async fn forever() -> Result<()> {
	let (_env, _tc) = TestEnv::new(Backend::Evm, false).await?;
	tracing::info!("Test env ready. Keeping live indefinitely...");
	loop {}
}
