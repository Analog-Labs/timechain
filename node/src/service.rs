//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use futures::channel::mpsc;
use sc_client_api::BlockBackend;
use sc_consensus_grandpa::SharedVoterState;
pub use sc_executor::NativeElseWasmExecutor;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_keystore::Keystore;
use std::{marker::PhantomData, sync::Arc, time::Duration};
use timechain_runtime::{self, opaque::Block, RuntimeApi};

// Our native executor instance.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
	/// Only enable the benchmarking host functions when we actually want to benchmark.
	#[cfg(feature = "runtime-benchmarks")]
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;
	/// Otherwise we only use the default Substrate host functions.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ExtendHostFunctions = ();

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		timechain_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		timechain_runtime::native_version()
	}
}

pub(crate) type FullClient =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport =
	sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;

#[allow(clippy::type_complexity)]
pub fn new_partial(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block, FullClient>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(
			sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
			sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
			sc_consensus_babe::BabeLink<Block>,
			Option<Telemetry>,
		),
	>,
	ServiceError,
> {
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = sc_service::new_native_or_wasm_executor(config);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let babe_config = sc_consensus_babe::configuration(&*client)?;
	let (block_import, babe_link) =
		sc_consensus_babe::block_import(babe_config, grandpa_block_import.clone(), client.clone())?;

	let slot_duration = babe_link.config().slot_duration();
	let justification_import = grandpa_block_import;

	let (import_queue, babe_task_handle) = sc_consensus_babe::import_queue(
		babe_link.clone(),
		block_import.clone(),
		Some(Box::new(justification_import)),
		client.clone(),
		select_chain.clone(),
		move |_, ()| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

			Ok((slot, timestamp))
		},
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	std::mem::forget(babe_task_handle);

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (block_import, grandpa_link, babe_link, telemetry),
	})
}

/// Builds a new service for a full client.
pub fn new_full(
	config: Configuration,
	connector_url: Option<String>,
	connector_blockchain: Option<String>,
	connector_network: Option<String>,
) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (block_import, grandpa_link, babe_link, mut telemetry),
	} = new_partial(&config)?;

	let mut net_config = sc_network::config::FullNetworkConfiguration::new(&config.network);

	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);

	net_config.add_notification_protocol(sc_consensus_grandpa::grandpa_peers_set_config(
		grandpa_protocol_name.clone(),
	));

	// registering time p2p gossip protocol
	net_config.add_notification_protocol(time_worker::time_protocol_name::time_peers_set_config(
		time_worker::time_protocol_name::gossip_protocol_name(),
	));

	let warp_sync = Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		grandpa_link.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_params: Some(sc_service::WarpSyncParams::WithProvider(warp_sync)),
			net_config,
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);

		// adding dev acc for signature pallet
		keystore_container
			.local_keystore()
			.sr25519_generate_new(time_primitives::SIG_KEY_TYPE, Some("//Alice"))
			.expect("Creating key with account Alice should succeed.");

		// adding dev acc for schedule pallet
		keystore_container
			.local_keystore()
			.sr25519_generate_new(time_primitives::SKD_KEY_TYPE, Some("//Alice"))
			.expect("Creating key with account Alice should succeed.");
	}

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks: Option<()> = None;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let keystore = keystore_container.keystore();

	let (sign_data_sender, sign_data_receiver) = mpsc::channel(400);
	let (tx_data_sender, tx_data_receiver) = mpsc::channel(400);
	let (gossip_data_sender, gossip_data_receiver) = mpsc::channel(400);
	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let keystore = keystore.clone();
		let sign_data_sender = sign_data_sender.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				kv: keystore.clone(),
				sign_data_sender: sign_data_sender.clone(),
			};
			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore,
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: rpc_extensions_builder,
		backend: backend.clone(),
		system_rpc_tx,
		tx_handler_controller,
		config,
		telemetry: telemetry.as_mut(),
		sync_service: sync_service.clone(),
	})?;
	if role.is_authority() {
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let slot_duration = babe_link.config().slot_duration();
		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.keystore(),
			client: client.clone(),
			select_chain,
			block_import,
			env: proposer_factory,
			sync_oracle: sync_service.clone(),
			justification_sync_link: sync_service.clone(),
			create_inherent_data_providers: move |_parent, ()| async move {
				let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

				let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

				Ok((slot, timestamp))
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			block_proposal_slot_portion: sc_consensus_babe::SlotProportion::new(2f32 / 3f32),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;

		task_manager
			.spawn_essential_handle()
			.spawn_blocking("aura", Some("block-authoring"), babe);
	}

	if enable_grandpa {
		// if the node isn't actively participating in consensus then it doesn't
		// need a keystore, regardless of which protocol we use below.
		let keystore = if role.is_authority() { Some(keystore_container.keystore()) } else { None };

		let grandpa_config = sc_consensus_grandpa::Config {
			// FIXME #1578 make this available through chainspec
			gossip_duration: Duration::from_millis(333),
			justification_period: 512,
			name: Some(name),
			observer_enabled: false,
			keystore: keystore.clone(),
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: grandpa_protocol_name,
		};

		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = sc_consensus_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network: network.clone(),
			voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			sync: sync_service.clone(),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_consensus_grandpa::run_grandpa_voter(grandpa_config)?,
		);
		// injecting our Worker
		let time_params = time_worker::TimeWorkerParams {
			runtime: client.clone(),
			client: client.clone(),
			backend: backend.clone(),
			gossip_network: network,
			kv: keystore_container.keystore(),
			_block: PhantomData::default(),
			sign_data_receiver,
			tx_data_sender: tx_data_sender.clone(),
			gossip_data_receiver,
			accountid: PhantomData,
			sync_service,
		};

		task_manager.spawn_essential_handle().spawn_blocking(
			"time-worker",
			None,
			time_worker::start_timeworker_gadget(time_params),
		);

		//Injecting event worker
		let event_params = event_worker::EventWorkerParams {
			runtime: client.clone(),
			backend: backend.clone(),
			kv: keystore_container.keystore(),
			_block: PhantomData::default(),
			sign_data_sender: sign_data_sender.clone(),
			tx_data_receiver,
			accountid: PhantomData,
			connector_url: connector_url.clone(),
			connector_blockchain: connector_blockchain.clone(),
			connector_network: connector_network.clone(),
		};

		task_manager.spawn_essential_handle().spawn_blocking(
			"event-worker",
			None,
			event_worker::start_eventworker_gadget(event_params),
		);

		let taskexecutor_params = Box::new(task_executor::TaskExecutorParams {
			runtime: client.clone(),
			backend: backend.clone(),
			kv: keystore_container.keystore(),
			_block: PhantomData::default(),
			sign_data_sender,
			accountid: PhantomData,
			connector_url: connector_url.clone(),
			connector_blockchain: connector_blockchain.clone(),
			connector_network: connector_network.clone(),
		});
		task_manager.spawn_essential_handle().spawn_blocking(
			"task-executor",
			None,
			task_executor::start_taskexecutor_gadget(taskexecutor_params),
		);

		let payabletaskexecutor_params = payable_task_executor::PayableTaskExecutorParams {
			runtime: client,
			backend,
			kv: keystore_container.keystore(),
			_block: PhantomData::default(),
			tx_data_sender,
			gossip_data_sender,
			accountid: PhantomData,
			connector_url,
			connector_blockchain,
			connector_network,
		};
		task_manager.spawn_essential_handle().spawn_blocking(
			"payable-task-executor",
			None,
			payable_task_executor::start_payabletaskexecutor_gadget(payabletaskexecutor_params),
		)
	}

	network_starter.start_network();
	Ok(task_manager)
}
