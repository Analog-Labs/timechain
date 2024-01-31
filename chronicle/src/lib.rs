use crate::shards::{TimeWorker, TimeWorkerParams};
use crate::tasks::executor::{TaskExecutor, TaskExecutorParams};
use crate::tasks::spawner::{TaskSpawner, TaskSpawnerParams};
use anyhow::Result;
use futures::channel::mpsc;
use futures::stream::BoxStream;
use std::path::PathBuf;
use std::sync::Arc;
use time_primitives::{NetworkId, Runtime};
use tracing::{event, span, Level};

mod gmp;
#[cfg(test)]
mod mock;
mod network;
mod shards;
mod tasks;

pub use crate::network::{
	create_iroh_network, Message, Network, NetworkConfig, PeerId, PROTOCOL_NAME,
};

pub const TW_LOG: &str = "chronicle";

pub struct ChronicleConfig {
	pub network_id: NetworkId,
	pub network_keyfile: Option<PathBuf>,
	pub network_port: Option<u16>,
	pub timechain_url: String,
	pub timechain_keyfile: PathBuf,
	pub target_url: String,
	pub target_keyfile: PathBuf,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
}

impl ChronicleConfig {
	pub fn network_config(&self) -> NetworkConfig {
		NetworkConfig {
			secret: self.network_keyfile.clone(),
			bind_port: self.network_port,
		}
	}
}

pub async fn run_chronicle(
	config: ChronicleConfig,
	network: Arc<dyn Network>,
	net_request: BoxStream<'static, (PeerId, Message)>,
	substrate: impl Runtime,
) -> Result<()> {
	let peer_id = network.peer_id();
	let span = span!(
		target: TW_LOG,
		Level::INFO,
		"run_chronicle",
		?peer_id,
	);
	event!(target: TW_LOG, parent: &span, Level::INFO, "PeerId {:?}", peer_id);

	let (chain, subchain) = substrate
		.get_network(config.network_id)
		.await?
		.ok_or(anyhow::anyhow!("Network Id not supported"))?;

	let (tss_tx, tss_rx) = mpsc::channel(10);
	let task_spawner_params = TaskSpawnerParams {
		tss: tss_tx,
		blockchain: chain,
		network: subchain,
		url: config.target_url,
		keyfile: config.target_keyfile,
		timegraph_url: config.timegraph_url,
		timegraph_ssk: config.timegraph_ssk,
		substrate: substrate.clone(),
	};
	let task_spawner = loop {
		match TaskSpawner::new(task_spawner_params.clone()).await {
			Ok(task_spawner) => break task_spawner,
			Err(error) => {
				event!(
					target: TW_LOG,
					parent: &span,
					Level::INFO,
					"Initializing wallet returned an error {:?}, retrying in one second",
					error
				);
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			},
		}
	};

	let task_executor = TaskExecutor::new(TaskExecutorParams {
		network: config.network_id,
		task_spawner,
		substrate: substrate.clone(),
	});

	let time_worker = TimeWorker::new(TimeWorkerParams {
		network,
		task_executor,
		substrate,
		tss_request: tss_rx,
		net_request,
	});

	time_worker.run(&span).await;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::Mock;

	pub struct MockChronicle {}

	impl MockChronicle {
		pub async fn new(network_id: NetworkId) -> Self {
			let config = ChronicleConfig {
				network_id,
				url: None,
				bind_port: None,
				pkarr_relay: None,
				timechain_keyfile: self.timechain_keyfile,
				network_keyfile: None,
				timegraph_url: None,
				timegraph_ssk: None,
			};
			let (network, network_requests) =
				chronicle::create_iroh_network(config.network_config()).await?;
			let subxt =
				SubxtClient::with_keyfile("ws://127.0.0.1:9944", &config.timechain_keyfile).await?;
			chronicle::run_chronicle(config, network, network_requests, subxt).await
		}
	}

	#[tokio::test]
	async fn chronicle_smoke() -> Result<()> {
		Ok(())
	}
}
