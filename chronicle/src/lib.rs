use crate::shards::{TimeWorker, TimeWorkerParams};
use crate::tasks::executor::{TaskExecutor, TaskExecutorParams};
use crate::tasks::spawner::{TaskSpawner, TaskSpawnerParams};
use anyhow::Result;
use futures::channel::mpsc;
use futures::stream::BoxStream;
use std::path::PathBuf;
use std::sync::Arc;
use time_primitives::TssSigningRequest;
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
					Level::INFO,
					"Initializing wallet returned an error {:?}, retrying in one second",
					error
				);
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			},
		}
	};
	run_chronicle_with_spawner(
		config.network_id,
		network,
		net_request,
		task_spawner,
		tss_rx,
		substrate,
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
	});
	time_worker.run(&span).await;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{
		AssignedTasks, Mock, MockNetwork, MockShard, MockTask, Networks, Shards, Tasks,
	};
	use std::{collections::HashMap, sync::Mutex, thread, time::Duration};
	use time_primitives::{
		AccountId, Function, ShardId, ShardStatus, TaskDescriptor, TaskId, TaskStatus,
	};

	async fn chronicle(mock: Mock, network_id: NetworkId) {
		tracing::info!("running chronicle ");
		let (network, network_requests) =
			create_iroh_network(NetworkConfig { secret: None, bind_port: None })
				.await
				.unwrap();
		let (_, tss_rx) = mpsc::channel(10);
		run_chronicle_with_spawner(
			network_id,
			network,
			network_requests,
			mock.clone(),
			tss_rx,
			mock.clone(),
		)
		.await
		.unwrap();
	}

	#[tokio::test]
	async fn chronicle_smoke() -> Result<()> {
		tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
		let mut mocks = vec![];
		let networks: Networks = Default::default();
		let shards: Shards = Default::default();
		let tasks: Tasks = Default::default();
		let assigned_tasks: AssignedTasks = Default::default();
		for id in 0..3 {
			let mock = Mock::new(
				id,
				networks.clone(),
				shards.clone(),
				tasks.clone(),
				assigned_tasks.clone(),
			);
			let network_id = mock.create_network("ethereum".into(), "dev".into());
			let cloned_mock = mock.clone();
			thread::spawn(move || {
				let rt = tokio::runtime::Runtime::new().unwrap();
				rt.block_on(chronicle(cloned_mock, network_id));
			});
			mocks.push((mock, network_id));
		}

		let (mock, network_id) = &mocks[0];
		let members: Vec<AccountId> = vec![[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()];
		let shard_id = mock.create_shard(members.clone(), 2);
		tracing::info!(
			"shard is here {:?}",
			mock.get_shards(Default::default(), &members.clone()[0]).await
		);
		tracing::info!(
			"shard is here {:?}",
			mocks[1].0.get_shards(Default::default(), &members[0]).await
		);
		loop {
			tracing::info!("waiting for shard");
			if mock.get_shard_status(Default::default(), shard_id).await.unwrap()
				== ShardStatus::Online
			{
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}
		tracing::info!("creating task");
		let task_id = mock.create_task(TaskDescriptor {
			owner: Some(mock.account_id().clone()),
			network: *network_id,
			cycle: 1,
			function: Function::SendMessage {
				address: Default::default(),
				gas_limit: Default::default(),
				salt: Default::default(),
				payload: Default::default(),
			},
			period: 0,
			start: 0,
			timegraph: None,
			shard_size: 3,
		});
		tracing::info!("assigning task");
		mock.assign_task(task_id, shard_id);
		loop {
			tracing::info!("waiting for task");
			let task = mock.task(task_id).unwrap();
			tracing::info!("task {:?}", task.status);
			if task.status != TaskStatus::Completed {
				tracing::info!("task phase {:?}", task.phase);
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			break;
		}
		Ok(())
	}
}
