use crate::shards::{TimeWorker, TimeWorkerParams};
use crate::tasks::TaskParams;
use anyhow::{Context, Result};
use futures::channel::mpsc;
use futures::stream::BoxStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tc_subxt::MetadataVariant;
use time_primitives::{ConnectorParams, IConnector, NetworkId, Runtime};
use tokio::time::sleep;
use tracing::{event, span, Level};

//#[cfg(test)]
//mod mock;
mod network;
mod shards;
mod tasks;

/// Re-exported items from the `network` module.
pub use crate::network::{
	create_iroh_network, Message, Network, NetworkConfig, PeerId, PROTOCOL_NAME,
};

/// Logging target for the Chronicle application.
pub const TW_LOG: &str = "chronicle";

/// Configuration structure for the Chronicle application.
pub struct ChronicleConfig {
	/// Identifier for the network.
	pub network_id: NetworkId,
	/// Optional path to a network key file.
	pub network_keyfile: Option<PathBuf>,
	/// Optional network port number.
	pub network_port: Option<u16>,
	/// Metadata for the timechain.
	pub timechain_metadata: MetadataVariant,
	/// URL for the timechain.
	pub timechain_url: String,
	/// Path to a timechain key file.
	pub timechain_keyfile: PathBuf,
	/// URL for the target.
	pub target_url: String,
	/// Path to a target key file.
	pub target_keyfile: PathBuf,
	/// Path to a cache for TSS key shares.
	pub tss_keyshare_cache: PathBuf,
	/// Minimum balance chronicle should have.
	pub target_min_balance: u128,
}

impl ChronicleConfig {
	/// Creates a `NetworkConfig` from the `ChronicleConfig` instance.
	pub fn network_config(&self) -> NetworkConfig {
		NetworkConfig {
			secret: self.network_keyfile.clone(),
			bind_port: self.network_port,
		}
	}
}

/// Runs the Chronicle application.
///
/// This function initializes the necessary components and starts the main
/// application loop for the Chronicle service. It sets up the network, task
/// spawner, and various workers needed to process tasks and communicate with
/// the blockchain.
///
/// # Arguments
///
/// * `config` - Configuration for the Chronicle application.
/// * `network` - Network instance.
/// * `net_request` - Stream of network requests.
/// * `substrate` - Substrate runtime instance.
///
/// # Returns
///
/// * `Result<()>` - Returns an empty result on success, or an error on failure.
pub async fn run_chronicle<C: IConnector>(
	config: ChronicleConfig,
	network: Arc<dyn Network>,
	net_request: BoxStream<'static, (PeerId, Message)>,
	substrate: impl Runtime,
) -> Result<()> {
	let mnemonic =
		std::fs::read_to_string(config.target_keyfile).context("failed to read target keyfile")?;
	// Initialize the blockchain network components.
	let (chain, subchain) = substrate
		.get_network(config.network_id)
		.await?
		.ok_or(anyhow::anyhow!("Network Id not supported"))?;

	// Create a channel for TSS requests.
	let (tss_tx, tss_rx) = mpsc::channel(10);
	let connector_params = ConnectorParams {
		network_id: config.network_id,
		blockchain: chain,
		network: subchain,
		url: config.target_url,
		mnemonic,
	};
	// Initialize the connector, retrying on failure.
	let connector = loop {
		match C::new(connector_params.clone()).await {
			Ok(connector) => break connector,
			Err(error) => {
				event!(
					target: TW_LOG,
					Level::INFO,
					"Initializing connector returned an error {:?}, retrying in one second",
					error
				);
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			},
		}
	};

	event!(
		target: TW_LOG,
		Level::INFO,
		"Target wallet address: {}",
		connector.format_address(connector.address()),
	);

	while connector.balance(connector.address()).await? < config.target_min_balance {
		sleep(Duration::from_secs(10)).await;
		tracing::warn!("Chronicle balance is too low, retrying...");
	}

	// Get the peer ID of the network and create a tracing span.
	let peer_id = network.peer_id();
	let span = span!(
		target: TW_LOG,
		Level::INFO,
		"run_chronicle",
		?peer_id,
	);
	event!(target: TW_LOG, parent: &span, Level::INFO, "PeerId {:?}", peer_id);

	// Initialize the task executor.
	let task_params = TaskParams::new(substrate.clone(), connector, tss_tx);

	// Initialize the time worker.
	let time_worker = TimeWorker::new(TimeWorkerParams {
		network,
		task_params,
		substrate,
		tss_request: tss_rx,
		net_request,
		tss_keyshare_cache: config.tss_keyshare_cache,
	});

	// Run the time worker with the created tracing span.
	time_worker.run(&span).await;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::Mock;
	use std::time::Duration;
	use time_primitives::traits::IdentifyAccount;
	use time_primitives::{AccountId, Function, Msg, ShardStatus, TaskDescriptor};

	/// Asynchronous test helper to run Chronicle.
	///
	/// This function sets up a mock network and runs the Chronicle application
	/// for testing purposes.
	///
	/// # Arguments
	///
	/// * `mock` - Mock instance for testing.
	/// * `network_id` - Identifier for the network.
	async fn chronicle(mut mock: Mock, network_id: NetworkId) {
		tracing::info!("running chronicle ");
		// Create a mock network and request stream.
		let (network, network_requests) =
			create_iroh_network(NetworkConfig { secret: None, bind_port: None })
				.await
				.unwrap();
		// Create a channel for TSS requests and set it up in the mock.
		let (tss_tx, tss_rx) = mpsc::channel(10);
		mock.with_tss(tss_tx);
		// Run the Chronicle application with the mock network.
		run_chronicle_with_spawner(
			network_id,
			network,
			network_requests,
			mock.clone(),
			tss_rx,
			mock.clone(),
			"/tmp".into(),
		)
		.await
		.unwrap();
	}

	/// Smoke test for the Chronicle application.
	///
	/// This test initializes the logger, sets up a mock network, and runs the
	/// Chronicle application in multiple threads to ensure basic functionality.
	///
	/// # Returns
	///
	/// * `Result<()>` - Returns an empty result on success, or an error on failure.
	#[tokio::test]
	async fn chronicle_smoke() -> Result<()> {
		env_logger::try_init().ok();
		std::panic::set_hook(Box::new(tracing_panic::panic_hook));

		let mock = Mock::default().instance(42);
		let network_id = mock.create_network("ethereum".into(), "dev".into());
		// Spawn multiple threads to run the Chronicle application.
		for id in 0..3 {
			let instance = mock.instance(id);
			std::thread::spawn(move || {
				let rt = tokio::runtime::Runtime::new().unwrap();
				rt.block_on(chronicle(instance, network_id));
			});
		}
		// Wait for members to register.
		loop {
			tracing::info!("waiting for members to register");
			if mock.members(network_id).len() < 3 {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}
		// Collect member accounts.
		let members: Vec<AccountId> = mock
			.members(network_id)
			.into_iter()
			.map(|(public, _)| public.into_account())
			.collect();
		// Create a shard.
		let shard_id = mock.create_shard(members.clone(), 2);
		// Wait for the shard to be online.
		loop {
			tracing::info!("waiting for shard");
			if mock.get_shard_status(Default::default(), shard_id).await.unwrap()
				!= ShardStatus::Online
			{
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}
		tracing::info!("creating task");
		// Create a task and assign it to the shard.
		let task_id = mock.create_task(TaskDescriptor {
			owner: Some(mock.account_id().clone()),
			network: network_id,
			function: Function::SendMessage { msg: Msg::default() },
			start: 0,
			shard_size: 3,
		});
		tracing::info!("assigning task");
		mock.assign_task(task_id, shard_id);
		// Wait for the task to complete.
		loop {
			tracing::info!("waiting for task");
			let task = mock.task(task_id).unwrap();
			if task.result.is_none() {
				tracing::info!("task phase {:?}", task.phase);
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}
		Ok(())
	}
}
