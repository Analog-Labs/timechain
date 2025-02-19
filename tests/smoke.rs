use futures::StreamExt;
use tc_cli::Tc;
use tracing_subscriber::filter::EnvFilter;

mod common;

use common::TestEnv;
use time_primitives::{Address, NetworkId};

const SRC: NetworkId = 2;
const DEST: NetworkId = 3;

async fn run_smoke(tc: &Tc, src_addr: Address, dest_addr: Address) {
	let mut blockstream = tc.finality_notification_stream();
	let (_, start) = blockstream.next().await.expect("expected block");
	let gas_limit = tc
		.estimate_message_gas_limit(DEST, dest_addr, SRC, src_addr, vec![])
		.await
		.unwrap();
	let gas_cost = tc.estimate_message_cost(SRC, DEST, gas_limit, vec![]).await.unwrap();

	let msg_id = tc
		.send_message(SRC, src_addr, DEST, dest_addr, gas_limit, gas_cost, vec![])
		.await
		.unwrap();

	let mut id = None;
	let (exec, end) = loop {
		let (_, end) = blockstream.next().await.expect("expected block");
		let trace = tc.message_trace(SRC, msg_id).await.unwrap();
		let exec = trace.exec.as_ref().map(|t| t.task);
		tracing::info!(target: "smoke_test", "waiting for message {}", hex::encode(msg_id));
		id = Some(tc.print_table(id, "message", vec![trace]).await.unwrap());
		if let Some(exec) = exec {
			break (exec, end);
		}
	};
	let blocks = tc.read_events_blocks(exec).await.unwrap();
	let msgs = tc.messages(DEST, dest_addr, blocks).await.unwrap();
	let msg = msgs
		.into_iter()
		.find(|msg| msg.message_id() == msg_id)
		.expect("failed to find message");
	tc.print_table(None, "message", vec![msg]).await.unwrap();
	tc.println(None, format!("received message after {} blocks", end - start))
		.await
		.unwrap();
}

#[tokio::test]
// Resembles tc-cli smoke test
async fn smoke() {
	let filter = EnvFilter::from_default_env()
		.add_directive("tc_cli=info".parse().unwrap())
		.add_directive("gmp_evm=info".parse().unwrap())
		.add_directive("smoke_test=info".parse().unwrap());
	tracing_subscriber::fmt().with_env_filter(filter).init();

	let env = TestEnv::spawn(true).await.expect("Failed to spawn Test Environment");

	let (src_addr, dest_addr) = env.setup(SRC, DEST).await.expect("failed to setup test");

	// Run smoke test
	run_smoke(&env.tc, src_addr, dest_addr).await;

	// Restart chronicles
	assert!(env
		.restart(vec!["chronicle-2-evm", "chronicle-3-evm"])
		.await
		.expect("Failed to restart chronicles"));

	// Re-run smoke test: should still work
	run_smoke(&env.tc, src_addr, dest_addr).await;
}
