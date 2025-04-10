use anyhow::{Context, Result};
use e2e_tests::{Backend, TestEnv, Tester};
use futures::StreamExt;
use std::collections::HashSet;
use time_primitives::BlockHash;

async fn test_gateway_payments(tc: Tester) -> Result<()> {
	let mut stream = tc.finality_notification_stream();
	// collect shard tasks
	let mut tasks = HashSet::new();
	let (block_hash, _) = tc.latest_block().await?;
	for shard in tc.shards(block_hash).await? {
		if let Some(batch) = shard.batch_register {
			let task = tc.batch(batch, block_hash).await?.task;
			tasks.insert((task, batch));
		}
	}
	// wait for shard batches to execute
	let mut block_hash: Option<BlockHash> = None;
	for (task, batch) in tasks {
		loop {
			let Some((hash, _)) = stream.next().await else {
				continue;
			};
			block_hash = Some(hash);
			if tc.is_task_executed(task, hash).await? {
				break;
			}
			tracing::info!("waiting for task {task} / batch {batch}");
		}
	}

	let block_hash = block_hash.context("Block hash not found")?;
	tc.assert_reimbursement(block_hash).await?;
	let total_funds = tc.total_gateway_funds()?;
	let total_balance = tc.total_gateway_balance(block_hash).await?;
	tc.println(None, format!("shard registration msgs cost {}$", total_funds - total_balance))
		.await?;
	let _ = tc.exec_smoke(0, 1, vec![42]).await?;
	let (block_hash, _) = tc.latest_block().await?;
	tc.assert_reimbursement(block_hash).await?;
	let total_balance_after = tc.total_gateway_balance(block_hash).await?;
	tc.println(None, format!("made {}$ of profit with msg", total_balance_after - total_balance))
		.await?;
	anyhow::ensure!(total_balance_after >= total_balance);
	Ok(())
}

#[tokio::test]
#[ignore]
async fn gateway_payments() -> Result<()> {
	let tc = Tester::new().await?;
	test_gateway_payments(tc).await
}

#[tokio::test]
async fn gateway_payments_evm_tss() -> Result<()> {
	let (_env, tc) = TestEnv::new(Backend::Evm, true).await?;
	test_gateway_payments(tc).await
}
