use alloy::primitives::utils::format_units;
use alloy::primitives::U256;
use alloy::providers::Provider;
use anyhow::Result;
use e2e_tests::{Backend, TestEnv, ANVIL_PORT};
use std::fs::File;
use std::io::Read;
use std::time::Duration;

mod common;

use common::*;

#[tokio::test]
async fn oats_wrapped_evm() -> Result<()> {
	const MINT_AMOUNT: u64 = TRANSFER_AMOUNT * 2;

	let (env, tc) = TestEnv::new(Backend::Evm, false).await?;

	// Load raw deployment txs
	let mut file = File::open("contracts/txs.raw")?;
	let mut txs_hex = String::new();
	file.read_to_string(&mut txs_hex)?;

	let [tx1_raw, tx2_raw, tx3_raw, tx4_raw] = txs_hex
		.split(",")
		.map(|s| s.trim())
		.filter_map(|s| hex::decode(s).ok())
		.collect::<Vec<_>>()
		.try_into()
		.unwrap();

	let mut contracts = vec![];

	// On every chain:
	//
	// + Deploy Proxy+Token: tx1, tx2;
	// + Mint some tokens;
	// + Upgrade to V2 implementation: tx3, tx4;
	// + Deploy Callee;
	for nw_id in tc.iter() {
		let port = env.chain_container(nw_id)?.get_host_port_ipv4(ANVIL_PORT).await?;
		let rpc = common::build_rpc(MINTER_KEY, port).await?;

		// Deploy Proxy+Token to every network: tx1, tx2;
		let rcp1 = rpc
			.send_raw_transaction(&tx1_raw)
			.await?
			.with_timeout(Some(Duration::from_secs(10)))
			.get_receipt()
			.await?;
		let rcp2 = rpc
			.send_raw_transaction(&tx2_raw)
			.await?
			.with_timeout(Some(Duration::from_secs(10)))
			.get_receipt()
			.await?;
		let impl_v1 = rcp1.contract_address.expect("no contract address");
		let proxy = rcp2.contract_address.expect("no contract address");

		tracing::info!("network {nw_id}: proxy deployed to {proxy}, tx: {}", rcp2.transaction_hash);
		tracing::info!(
			"network {nw_id}: impl v1 deployed to {impl_v1}, tx: {}",
			rcp1.transaction_hash
		);

		// Mint some tokens;
		let v1 = IERC20::new(proxy, rpc.clone());
		let _rcp = v1.mint(MINTER, U256::from(MINT_AMOUNT)).send().await?.get_receipt().await?;

		let bal = v1.balanceOf(MINTER).call().await?;
		assert_eq!(bal, U256::from(MINT_AMOUNT));
		let supply = v1.totalSupply().call().await?;
		assert_eq!(supply, U256::from(MINT_AMOUNT));

		let decimals = v1.decimals().call().await?;
		let ticker = v1.symbol().call().await?;
		tracing::info!(
			"network {nw_id}: minted {:.6} {ticker} to {MINTER}",
			format_units(bal, decimals)?
		);

		// v1 does not have cap() method,
		// therefore this should fail
		assert!(v1.cap().call().await.is_err());

		// Upgrade to V2 implementation: tx3, tx4
		tracing::info!("network {nw_id}: upgrading token to impl v2");

		let rcp3 = rpc
			.send_raw_transaction(&tx3_raw)
			.await?
			.with_timeout(Some(Duration::from_secs(10)))
			.get_receipt()
			.await?;
		let rcp4 = rpc
			.send_raw_transaction(&tx4_raw)
			.await?
			.with_timeout(Some(Duration::from_secs(10)))
			.get_receipt()
			.await?;
		let impl_v2 = rcp3.contract_address.expect("no contract address");
		tracing::info!(
			"network {nw_id}: impl v2 deployed to {impl_v2}, tx: {}",
			rcp3.transaction_hash
		);
		tracing::info!("network {nw_id}: token upgraded to impl v2, tx: {}", rcp4.transaction_hash);

		// We query the same contract which is proxy,
		// but its implementation is now upgraded to v2.
		let v2 = v1;
		// v2 now has cap() method
		assert_eq!(v2.cap().call().await?, U256::from(CAP_AMOUNT));
		// Balances should stay unchanged
		assert_eq!(v2.balanceOf(MINTER).call().await?, bal);
		assert_eq!(v2.totalSupply().call().await?, supply);

		// Deploy Callee;
		let callee = Callee::deploy(rpc.clone(), proxy).await?;

		contracts.push((
			nw_id,
			OATSSender::new(proxy, rpc.clone()),
			OATSSenderCaller::new(proxy, rpc.clone()),
			callee,
		));
	}

	tracing::info!("Testing OATS Send flow");
	let senders = contracts.clone().into_iter().map(|(n, s, _, _)| (n, s)).collect();
	common::test_oats_sender(senders, &tc).await?;

	tracing::info!("Testing OATS Send+Call flow");
	let callers = contracts.into_iter().map(|(n, _, f, t)| (n, f, t)).collect();
	common::test_oats_sender_caller(callers, &tc).await
}
