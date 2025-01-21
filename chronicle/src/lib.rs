use crate::admin::AdminMsg;
use crate::network::{create_iroh_network, NetworkConfig};
use crate::runtime::Runtime;
use crate::shards::{TimeWorker, TimeWorkerParams};
use crate::tasks::TaskParams;
use anyhow::Result;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use gmp::Backend;
use scale_codec::Decode;
use std::path::PathBuf;
use std::sync::Arc;
use time_primitives::admin::Config;
use time_primitives::{ConnectorParams, NetworkId};
use tracing::{span, Level};

pub mod admin;
#[cfg(test)]
mod mock;
mod network;
mod runtime;
mod shards;
mod tasks;

pub fn init_logger() {
	let filter = tracing_subscriber::EnvFilter::from_default_env()
		.add_directive("chronicle=debug".parse().unwrap())
		.add_directive("tss=debug".parse().unwrap());
	tracing_subscriber::fmt()
		.pretty()
		.with_ansi(false)
		.with_max_level(tracing::Level::INFO)
		.with_file(true)
		.with_line_number(true)
		.with_env_filter(filter)
		.try_init()
		.ok();
	std::panic::set_hook(Box::new(tracing_panic::panic_hook));
}

/// Configuration structure for the Chronicle application.
pub struct ChronicleConfig {
	/// Identifier for the network.
	pub network_id: NetworkId,
	/// Network private key.
	pub network_key: [u8; 32],
	/// URL for the target.
	pub target_url: String,
	/// Path to a target key file.
	pub target_mnemonic: String,
	/// Path to a cache for TSS key shares.
	pub tss_keyshare_cache: PathBuf,
	/// Backend
	pub backend: Backend,
	// Cctp request sender
	pub cctp_sender: Option<String>,
	// Cctp attestation url
	pub cctp_attestation: Option<String>,
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
pub async fn run_chronicle(
	config: ChronicleConfig,
	substrate: Arc<dyn Runtime>,
	mut admin: mpsc::Sender<AdminMsg>,
) -> Result<()> {
	let mut ticker = substrate.finality_notification_stream();
	// Initialize connector
	let (chain, subchain) = loop {
		let network = substrate.get_network(config.network_id).await?;
		if let Some(network) = network {
			break network;
		}
		tracing::warn!("network {} isn't registered", config.network_id);
		ticker.next().await;
	};
	let (tss_tx, tss_rx) = mpsc::channel(10);
	let blockchain = String::decode(&mut chain.0.to_vec().as_slice()).unwrap_or_default();
	let network = String::decode(&mut subchain.0.to_vec().as_slice()).unwrap_or_default();
	let connector_params = ConnectorParams {
		network_id: config.network_id,
		blockchain,
		network,
		url: config.target_url,
		mnemonic: config.target_mnemonic,
		cctp_sender: config.cctp_sender,
		cctp_attestation: config.cctp_attestation,
	};
	let connector = loop {
		match config.backend.connect(&connector_params).await {
			Ok(connector) => break connector,
			Err(error) => {
				tracing::info!(
					"Initializing connector returned an error {:?}, retrying in one second",
					error
				);
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			},
		}
	};

	// initialize networking
	let (network, network_requests) =
		create_iroh_network(NetworkConfig { secret: config.network_key }).await?;

	// initialize wallets
	let timechain_address = time_primitives::format_address(substrate.account_id());
	let target_address = connector.format_address(connector.address());
	let peer_id = network.format_peer_id(network.peer_id());
	let span = span!(
		Level::INFO,
		"chronicle",
		timechain = timechain_address,
		target = target_address,
		peer_id = peer_id,
	);
	admin
		.send(AdminMsg::SetConfig(Config {
			network: config.network_id,
			account: timechain_address,
			public_key: substrate.public_key().clone(),
			address: target_address,
			peer_id,
			peer_id_hex: hex::encode(network.peer_id()),
		}))
		.await?;
	loop {
		if substrate.is_registered().await? {
			break;
		}
		tracing::warn!(parent: &span, "chronicle isn't registered");
		ticker.next().await;
	}

	let task_params = TaskParams::new(substrate.clone(), connector, tss_tx);
	let time_worker = TimeWorker::new(TimeWorkerParams {
		network,
		task_params,
		substrate,
		tss_request: tss_rx,
		net_request: network_requests,
		tss_keyshare_cache: config.tss_keyshare_cache,
		admin_request: admin.clone(),
	});
	time_worker.run(&span).await;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::Mock;
	use futures::{Future, FutureExt, StreamExt};
	use polkadot_sdk::sp_runtime::BoundedVec;
	use scale_codec::Encode;
	use std::time::Duration;
	use time_primitives::traits::IdentifyAccount;
	use time_primitives::{AccountId, ChainName, ChainNetwork, ShardStatus, Task};

	/// Asynchronous test helper to run Chronicle.
	///
	/// This function sets up a mock network and runs the Chronicle application
	/// for testing purposes.
	///
	/// # Arguments
	///
	/// * `mock` - Mock instance for testing.
	/// * `network_id` - Identifier for the network.
	async fn chronicle(mock: Mock, network_id: NetworkId, exit: impl Future<Output = ()> + Unpin) {
		tracing::info!("running chronicle");
		let network_key = *mock.account_id().as_ref();
		let (tx, mut rx) = mpsc::channel(10);
		let root = if std::env::var("CI").is_ok() { "." } else { "/tmp" };
		let tss_keyshare_cache = format!("{root}/chronicles/{}", hex::encode(network_key)).into();
		std::fs::create_dir_all(&tss_keyshare_cache).unwrap();
		let handle = tokio::task::spawn(run_chronicle(
			ChronicleConfig {
				network_id,
				network_key,
				target_url: "tempfile".to_string(),
				target_mnemonic: "mnemonic".into(),
				tss_keyshare_cache,
				backend: Backend::Rust,
				cctp_sender: None,
				cctp_attestation: None,
			},
			Arc::new(mock.clone()),
			tx,
		));

		tokio::spawn(async move {
			while let Some(msg) = rx.next().await {
				if let AdminMsg::SetConfig(config) = msg {
					tracing::info!("received chronicle config");
					mock.register_member(
						network_id,
						config.public_key,
						hex::decode(&config.peer_id_hex).unwrap().try_into().unwrap(),
						0,
					);
					tracing::info!("registered chronicle");
				}
			}
		});
		tracing::info!("registered chronicle");
		futures::future::select(handle, exit).await;
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
		init_logger();

		let mock = Mock::default().instance(42);
		let network_id = mock.create_network(
			ChainName(BoundedVec::truncate_from("rust".encode())),
			ChainNetwork(BoundedVec::truncate_from("rust".encode())),
		);
		// Spawn multiple threads to run the Chronicle application.
		for id in 0..3 {
			let instance = mock.instance(id);
			std::thread::spawn(move || {
				let rt = tokio::runtime::Runtime::new().unwrap();
				rt.block_on(chronicle(instance, network_id, futures::future::pending::<()>()));
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
			if mock.get_shard_status(shard_id).await.unwrap() != ShardStatus::Online {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}

		tracing::info!("creating task");
		// Create a task and assign it to the shard.
		let task_id = mock.create_task(Task::ReadGatewayEvents { blocks: 0..1 });
		tracing::info!("assigning task");
		mock.assign_task(task_id, shard_id);
		// Wait for the task to complete.
		loop {
			tracing::info!("waiting for task");
			let task = mock.task(task_id).unwrap();
			if task.result.is_none() {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}
		Ok(())
	}

	#[tokio::test]
	async fn chronicle_restart() -> Result<()> {
		init_logger();

		let mock = Mock::default().instance(42);
		let network_id = mock.create_network(
			ChainName(BoundedVec::truncate_from("rust".encode())),
			ChainNetwork(BoundedVec::truncate_from("rust".encode())),
		);
		let mut shutdown = vec![];
		// Spawn multiple threads to run the Chronicle application.
		for id in 0..3 {
			let instance = mock.instance(id + 4);
			let (tx, rx) = futures::channel::oneshot::channel();
			shutdown.push(tx);
			std::thread::spawn(move || {
				let rt = tokio::runtime::Runtime::new().unwrap();
				rt.block_on(chronicle(instance, network_id, rx.map(|_| ())));
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
			if mock.get_shard_status(shard_id).await.unwrap() != ShardStatus::Online {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}

		for tx in shutdown {
			tx.send(()).unwrap();
		}
		// Spawn multiple threads to run the Chronicle application.
		for id in 0..3 {
			let instance = mock.instance(id + 4);
			std::thread::spawn(move || {
				let rt = tokio::runtime::Runtime::new().unwrap();
				rt.block_on(chronicle(instance, network_id, futures::future::pending()));
			});
		}

		tracing::info!("creating task");
		// Create a task and assign it to the shard.
		let task_id = mock.create_task(Task::ReadGatewayEvents { blocks: 0..1 });
		tracing::info!("assigning task");
		mock.assign_task(task_id, shard_id);
		// Wait for the task to complete.
		loop {
			tracing::info!("waiting for task");
			let task = mock.task(task_id).unwrap();
			if task.result.is_none() {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}
		Ok(())
	}
}
