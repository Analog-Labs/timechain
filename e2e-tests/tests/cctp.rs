use anyhow::Result;
use e2e_tests::{Backend, TestEnvBuilder};
use time_primitives::CCTPMessage;

async fn cctp(backend: Backend, shard_size: u16) -> Result<()> {
	let tc = TestEnvBuilder::setup(backend, shard_size, shard_size).await?;
	let (block_hash, _) = tc.latest_block().await?;
	let src_addr = tc.tester(0)?;
	let dest_addr = tc.tester(1)?;
	tc.set_network_config(0, Some(src_addr), block_hash).await?;
	tc.set_network_config(1, Some(dest_addr), block_hash).await?;
	let cctp_msg_data = "0000000000000000000000060000000000040CDD0000000000000000000000009F3B8679C73C2FEF8B59B4F3444D4E156FB70AA50000000000000000000000009F3B8679C73C2FEF8B59B4F3444D4E156FB70AA50000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001C7D4B196CB0C7B01D743FBC6116A902379C723800000000000000000000000033A2838EABD69A081CBEBE3F11DED4086C1CFC25000000000000000000000000000000000000000000000000000000000098968000000000000000000000000033A2838EABD69A081CBEBE3F11DED4086C1CFC25";
	let msg_data =
		hex::decode(cctp_msg_data).expect("Unable to create msg data from dummy cctp msg");
	let cctp_payload = CCTPMessage {
		attestation: vec![],
		message: msg_data,
		extra_data: [0u8; 32].to_vec(),
	};
	let msg = tc.smoke_test(cctp_payload.encode()).await?;
	let attested = CCTPMessage::from_bytes(&msg.bytes).map_err(|e| anyhow::anyhow!("{:?}", e))?;
	assert!(!attested.attestation.is_empty());
	assert!(attested.extra_data == cctp_payload.extra_data);
	Ok(())
}

#[tokio::test]
async fn cctp_evm_tss() -> Result<()> {
	cctp(Backend::Evm, 3).await
}
