use anyhow::{Context, Result};
use std::path::Path;
use std::process;
use tc_cli::{Sender, Tc};
use time_primitives::{Address, NetworkId};

pub struct TestEnv {
	pub tc: Tc,
}

const CONFIG: &str = "local-evm-e2e.yaml";
const ENV: &str = "config/envs/local";

impl TestEnv {
	async fn new() -> Result<Self> {
		let sender = Sender::new();
		let tc = Tc::new(Path::new(ENV).to_path_buf(), CONFIG, sender)
			.await
			.context("Error creating Tc client")?;
		Ok(TestEnv { tc })
	}

	/// spawns new testing env
	pub async fn spawn(build: bool) -> Result<Self> {
		if build && !build_containers()? {
			anyhow::bail!("Failed to build containers");
		}

		if !docker_up()? {
			anyhow::bail!("Failed to start containers");
		}
		Self::new().await
	}

	/// sets up test
	pub async fn setup(&self, src: NetworkId, dest: NetworkId) -> Result<(Address, Address)> {
		self.tc.setup_test(src, dest).await
	}

	/// restart container
	pub async fn restart(&self, containers: Vec<&str>) -> Result<bool> {
		docker_restart(containers)
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
	let mut cmd = process::Command::new(Path::new("scripts/build_docker.sh"));
	let mut child = cmd.spawn().context("Error building containers")?;

	child.wait().map(|c| c.success()).context("Error building containers: {e}")
}

fn docker_up() -> Result<bool> {
	let mut cmd = process::Command::new("docker");
	cmd.arg("compose").arg("--profile=evm").arg("up").arg("-d").arg("--wait");

	let mut child = cmd.spawn().context("Error starting containers")?;
	// Wait for all containers to start
	child.wait().map(|c| c.success()).context("Error starting containers")
}

fn docker_down() -> Result<bool> {
	let mut cmd = process::Command::new("docker");
	cmd.arg("compose").arg("--profile=evm").arg("down");

	let mut child = cmd.spawn().context("Error stopping containers")?;
	// Wait for all containers to start
	child.wait().map(|c| c.success()).context("Error stopping containers: {e}")
}

fn docker_restart(containers: Vec<&str>) -> Result<bool> {
	let mut cmd = process::Command::new("docker");
	cmd.arg("compose").arg("stop").args(containers.as_slice());

	let mut child = cmd.spawn().context("Error stopping containers")?;
	// wait for the containers to stop
	child.wait().map(|c| c.success()).context("Error stopping containers")?;
	let mut cmd = process::Command::new("docker");
	cmd.arg("compose").arg("start").args(containers.as_slice());
	let mut child = cmd.spawn().context("Error stopping containers")?;
	// wait for the containers to start
	child.wait().map(|c| c.success()).context("Error starting containers")
}
