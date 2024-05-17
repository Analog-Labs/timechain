use crate::shards::{TimeWorker, TimeWorkerParams};
use crate::tasks::executor::{TaskExecutor, TaskExecutorParams};
use crate::tasks::spawner::{TaskSpawner, TaskSpawnerParams};
use anyhow::Result;
use futures::channel::mpsc;
use futures::stream::BoxStream;
use std::path::PathBuf;
use std::sync::Arc;
use time_primitives::{NetworkId, Runtime, TssSigningRequest};
use tracing::{event, span, Level};

mod metrics;

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
	pub tss_keyshare_cache: PathBuf,
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
	let (chain, subchain) = substrate
		.get_network(config.network_id)
		.await?
		.ok_or(anyhow::anyhow!("Network Id not supported"))?;

	let (tss_tx, tss_rx) = mpsc::channel(10);
	let task_spawner_params = TaskSpawnerParams {
		tss: tss_tx,
		blockchain: chain,
		network: subchain,
		network_id: config.network_id,
		url: config.target_url,
		keyfile: config.target_keyfile,
		substrate: substrate.clone(),
	};
	let task_spawner = loop {
		match TaskSpawner::new(task_spawner_params.clone()).await {
			Ok(task_spawner) => break task_spawner,
			Err(error) => {
				event!(
					target: TW_LOG,
					Level::INFO,
					"Initializing wallet returned an error {:?}, retrying in one second",
					error
				);
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			},
		}
	};

	tracing::info!("Target wallet address: {:?}", task_spawner.target_address());

	run_chronicle_with_spawner(
		config.network_id,
		network,
		net_request,
		task_spawner,
		tss_rx,
		substrate,
		config.tss_keyshare_cache,
	)
	.await
}

async fn run_chronicle_with_spawner(
	network_id: NetworkId,
	network: Arc<dyn Network>,
	net_request: BoxStream<'static, (PeerId, Message)>,
	task_spawner: impl crate::tasks::TaskSpawner,
	tss_request: mpsc::Receiver<TssSigningRequest>,
	substrate: impl Runtime,
	tss_keyshare_cache: PathBuf,
) -> Result<()> {
	let peer_id = network.peer_id();
	let span = span!(
		target: TW_LOG,
		Level::INFO,
		"run_chronicle",
		?peer_id,
	);
	event!(target: TW_LOG, parent: &span, Level::INFO, "PeerId {:?}", peer_id);

	let task_executor = TaskExecutor::new(TaskExecutorParams {
		network: network_id,
		task_spawner,
		substrate: substrate.clone(),
	});
	let time_worker = TimeWorker::new(TimeWorkerParams {
		network,
		task_executor,
		substrate,
		tss_request,
		net_request,
		tss_keyshare_cache,
	});
	time_worker.run(&span).await;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::Mock;
	use std::time::Duration;
	use time_primitives::sp_runtime::traits::IdentifyAccount;
	use time_primitives::{AccountId, Function, Msg, ShardStatus, TaskDescriptor};

	async fn chronicle(mut mock: Mock, network_id: NetworkId) {
		tracing::info!("running chronicle ");
		let (network, network_requests) =
			create_iroh_network(NetworkConfig { secret: None, bind_port: None })
				.await
				.unwrap();
		let (tss_tx, tss_rx) = mpsc::channel(10);
		mock.with_tss(tss_tx);
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

	#[tokio::test]
	async fn chronicle_smoke() -> Result<()> {
		env_logger::try_init().ok();
		std::panic::set_hook(Box::new(tracing_panic::panic_hook));

		let mock = Mock::default().instance(42);
		let network_id = mock.create_network("ethereum".into(), "dev".into());
		for id in 0..3 {
			let instance = mock.instance(id);
			std::thread::spawn(move || {
				let rt = tokio::runtime::Runtime::new().unwrap();
				rt.block_on(chronicle(instance, network_id));
			});
		}
		loop {
			tracing::info!("waiting for members to register");
			if mock.members(network_id).len() < 3 {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}
		let members: Vec<AccountId> = mock
			.members(network_id)
			.into_iter()
			.map(|(public, _)| public.into_account())
			.collect();
		let shard_id = mock.create_shard(members.clone(), 2);
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
		let task_id = mock.create_task(TaskDescriptor {
			owner: Some(mock.account_id().clone()),
			network: network_id,
			function: Function::SendMessage { msg: Msg::default() },
			start: 0,
			shard_size: 3,
		});
		tracing::info!("assigning task");
		mock.assign_task(task_id, shard_id);
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
