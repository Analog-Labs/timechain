use anyhow::{Context, Result};
use std::path::Path;
use std::process;
use tc_cli::{Sender, Tc};

pub struct TestEnv {
	tc: Tc,
}

const CONFIG: &str = "local-evm.yaml";
const ENV: &str = "../config/envs/local";

impl TestEnv {
	async fn new() -> Result<Self> {
		let sender = Sender::new();
		let tc = Tc::new(Path::new(ENV).to_path_buf(), CONFIG, sender)
			.await
			.context("Error creating Tc client")?;
		Ok(TestEnv { tc })
	}

	pub async fn spawn() -> Result<Self> {
		// if !build_containers()? {
		//	anyhow::bail!("Failed to build containers");
		// }

		if !docker_up()? {
			anyhow::bail!("Failed to start containers");
		}
		Self::new().await
	}
}

impl Drop for TestEnv {
	/// Tear-down logic for the tests
	fn drop(&mut self) {
		if !docker_down().expect("Failed to stop containers") {
			println!(
				"Failed to stop containers, please stop by hand with:\n\
                       \t $> docker compose --profile=ethereum down"
			);
		};
	}
}

fn build_containers() -> Result<bool> {
	let mut cmd = process::Command::new(Path::new("../scripts/build_docker.sh"));
	let mut child = cmd.spawn().context("Error building containers")?;

	child.wait().map(|c| c.success()).context("Error building containers: {e}")
}

fn docker_up() -> Result<bool> {
	let mut cmd = process::Command::new("docker");

	cmd.arg("compose").arg("--profile=ethereum").arg("up").arg("--wait");

	let mut child = cmd.spawn().context("Error starting containers")?;

	// Wait for all containers to start
	child.wait().map(|c| c.success()).context("Error starting containers")
}

fn docker_down() -> Result<bool> {
	let mut cmd = process::Command::new("docker");

	cmd.arg("compose").arg("--profile=ethereum").arg("down");

	let mut child = cmd.spawn().context("Error stopping containers")?;

	// Wait for all containers to start
	child.wait().map(|c| c.success()).context("Error stopping containers: {e}")
}
