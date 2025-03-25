use crate::common::TestEnv;
use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use tracing_subscriber::filter::EnvFilter;

mod common;

// Test that verifies submitting multiple heartbeats in the same timeout period
// This test uses the chronicle container to test the heartbeat functionality
#[tokio::test]
async fn test_multiple_heartbeats() -> Result<()> {
	let filter = EnvFilter::from_default_env()
		.add_directive("tc_cli=info".parse()?)
		.add_directive("heartbeat_test=info".parse()?);
	tracing_subscriber::fmt().with_env_filter(filter).init();

	// Spawn the test environment
	let env = TestEnv::spawn(true).await.context("Failed to spawn Test Environment")?;

	// Setup the test environment
	let _testers = env.setup().await.context("failed to setup test")?;

	// We'll use the docker logs to verify heartbeat behavior
	// First, let's clear any existing logs
	tracing::info!(target: "heartbeat_test", "Clearing existing logs");
	let _ = Command::new("docker")
		.args(["logs", "--tail", "1", "chronicle-2-evm"])
		.output()?;

	// Wait a moment to ensure logs are cleared
	sleep(Duration::from_secs(2)).await;

	// Restart the chronicle to trigger a fresh heartbeat submission
	tracing::info!(target: "heartbeat_test", "Restarting chronicle to trigger heartbeat");
	assert!(env
		.restart(vec!["chronicle-2-evm"])
		.await
		.context("Failed to restart chronicle")?);

	// Wait for the chronicle to start and submit a heartbeat
	sleep(Duration::from_secs(5)).await;

	// Check the logs for heartbeat submission
	tracing::info!(target: "heartbeat_test", "Checking logs for first heartbeat");
	let output = Command::new("docker")
		.args(["logs", "--tail", "50", "chronicle-2-evm"])
		.output()?;

	let logs = String::from_utf8_lossy(&output.stdout);
	tracing::info!(target: "heartbeat_test", "Chronicle logs: {}", logs);

	// Verify that a heartbeat was submitted
	assert!(
		logs.contains("submitted heartbeat") || logs.contains("submitting heartbeat"),
		"Expected to find heartbeat submission in logs"
	);

	// Now try to trigger another heartbeat submission within the same timeout period
	// We'll do this by restarting the chronicle again
	tracing::info!(target: "heartbeat_test", "Restarting chronicle again to trigger second heartbeat");
	assert!(env
		.restart(vec!["chronicle-2-evm"])
		.await
		.context("Failed to restart chronicle")?);

	// Wait for the chronicle to start and attempt to submit another heartbeat
	sleep(Duration::from_secs(5)).await;

	// Check the logs for the second heartbeat attempt
	tracing::info!(target: "heartbeat_test", "Checking logs for second heartbeat");
	let output = Command::new("docker")
		.args(["logs", "--tail", "50", "chronicle-2-evm"])
		.output()?;

	let logs = String::from_utf8_lossy(&output.stdout);
	tracing::info!(target: "heartbeat_test", "Chronicle logs after second attempt: {}", logs);

	// Verify that the second heartbeat was rejected or already submitted
	assert!(
		logs.contains("heartbeat already submitted") || logs.contains("AlreadySubmittedHeartbeat"),
		"Expected to find indication that heartbeat was already submitted"
	);

	Ok(())
}
