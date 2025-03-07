use crate::common::TestEnv;
use anyhow::{Context, Result};
use futures::StreamExt;
use tc_cli::Tc;
use time_primitives::{Address, NetworkId};
use tracing_subscriber::filter::EnvFilter;

mod common;

const SRC: NetworkId = 2;
const DEST: NetworkId = 3;

async fn run_smoke(tc: &Tc, src_addr: Address, dest_addr: Address) -> Result<()> {
	let mut blockstream = tc.finality_notification_stream();
	let (_, start) = blockstream.next().await.context("expected block")?;
	let gas_limit = tc.estimate_message_gas_limit(DEST, dest_addr, SRC, src_addr, vec![]).await?;
	let gas_cost = tc.estimate_message_cost(SRC, DEST, gas_limit, vec![]).await?;

	let msg_id = tc
		.send_message(SRC, src_addr, DEST, dest_addr, gas_limit, gas_cost, vec![])
		.await?;

	let mut id = None;
	let (exec, end) = loop {
		let (_, end) = blockstream.next().await.context("expected block")?;
		let trace = tc.message_trace(SRC, msg_id).await?;
		let exec = trace.exec.as_ref().map(|t| t.task);
		tracing::info!(target: "smoke_test", "waiting for message {}", hex::encode(msg_id));
		id = Some(tc.print_table(id, "message", vec![trace]).await?);
		if let Some(exec) = exec {
			break (exec, end);
		}
	};
	let blocks = tc.read_events_blocks(exec).await?;
	let msgs = tc.messages(DEST, dest_addr, blocks).await?;
	let msg = msgs
		.into_iter()
		.find(|msg| msg.message_id() == msg_id)
		.expect("failed to find message");
	tc.print_table(None, "message", vec![msg]).await?;
	tc.println(None, format!("received message after {} blocks", end - start))
		.await?;

	Ok(())
}

#[tokio::test]
// Resembles tc-cli smoke test
async fn smoke() -> Result<()> {
	let filter = EnvFilter::from_default_env()
		.add_directive("tc_cli=info".parse()?)
		.add_directive("gmp_evm=info".parse()?)
		.add_directive("smoke_test=info".parse()?);
	tracing_subscriber::fmt().with_env_filter(filter).init();

	let env = TestEnv::spawn(true).await.context("Failed to spawn Test Environment")?;

	let testers = env.setup().await.context("failed to setup test")?;
	let src_addr = testers.get(&SRC).context("tester src contract not found")?.0;
	let dest_addr = testers.get(&DEST).context("tester dest contract not found")?.0;

	// Run smoke test
	run_smoke(&env.tc, src_addr, dest_addr).await?;

	// Restart chronicles
	assert!(env
		.restart(vec!["chronicle-2-evm", "chronicle-3-evm"])
		.await
		.context("Failed to restart chronicles")?);

	// Re-run smoke test: should still work
	run_smoke(&env.tc, src_addr, dest_addr).await?;

	Ok(())
}
