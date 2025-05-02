use anyhow::Result;
use e2e_tests::{TestEnv, Tester, TestingBackend};
use time_primitives::CCTPMessage;

async fn test_cctp(mut tc: Tester) -> Result<()> {
	let src_addr = tc.tester(0)?.0;
	let dest_addr = tc.tester(1)?.0;
	tc.add_cctp_contract(0, src_addr)?;
	tc.add_cctp_contract(1, dest_addr)?;
	let (block_hash, _) = tc.latest_block().await?;
	tc.set_network_config(0, block_hash).await?;
	tc.set_network_config(1, block_hash).await?;
	let cctp_msg_data = "0000000000000000000000060000000000040CDD0000000000000000000000009F3B8679C73C2FEF8B59B4F3444D4E156FB70AA50000000000000000000000009F3B8679C73C2FEF8B59B4F3444D4E156FB70AA50000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001C7D4B196CB0C7B01D743FBC6116A902379C723800000000000000000000000033A2838EABD69A081CBEBE3F11DED4086C1CFC25000000000000000000000000000000000000000000000000000000000098968000000000000000000000000033A2838EABD69A081CBEBE3F11DED4086C1CFC25";
	let msg_data =
		hex::decode(cctp_msg_data).expect("Unable to create msg data from dummy cctp msg");
	let cctp_payload = CCTPMessage {
		attestation: vec![],
		message: msg_data,
		extra_data: [0u8; 32].to_vec(),
	};
	let msg = tc.exec_smoke(0, 1, cctp_payload.encode()).await?;
	let attested = CCTPMessage::from_bytes(&msg.bytes).map_err(|e| anyhow::anyhow!("{:?}", e))?;
	assert!(!attested.attestation.is_empty());
	assert!(attested.extra_data == cctp_payload.extra_data);
	Ok(())
}

async fn test_zenswap(mut tc: Tester) -> Result<()> {
	let src = 0;
	let dst = 1;
	let (block, _) = tc.latest_block().await?;
	let (zen, plug) = tc.deploy_zenswap(src, block).await?;
	let (d_zen, d_plug) =
		if src != dst { tc.deploy_zenswap(dst, block).await? } else { (zen, plug) };

	tc.add_cctp_contract(src, plug)?;
	let (block_hash, _) = tc.latest_block().await?;
	tc.set_network_config(src, block_hash).await?;
	let msg_id = tc.send_swap(src, dst, zen, plug, d_zen, d_plug).await?;
	tracing::info!("swap sent with msg_id: {:?}", msg_id);
	Ok(())
}

#[tokio::test]
#[ignore]
async fn cctp() -> Result<()> {
	let tc = Tester::new().await?;
	test_cctp(tc).await
}

#[tokio::test]
async fn cctp_evm() -> Result<()> {
	let (_env, tc) = TestEnv::new(TestingBackend::evm_local(), false).await?;
	test_cctp(tc).await
}

#[tokio::test]
async fn zenswap_evm() -> Result<()> {
	let (_env, tc) = TestEnv::new(
		TestingBackend::evm_fork("https://sepolia.drpc.org".into(), 8226023u64),
		false,
	)
	.await?;
	test_zenswap(tc).await
}
