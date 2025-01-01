#![cfg_attr(not(feature = "std"), no_std)]
//! # Timechain Task Pallet
//!
//! This chart shows all the extrinsics and events of the task pallet. It
//! categorizes the extrinsics into two types: those that can be called by
//! root or council, and those that can be called by any user. The root or
//! council extrinsics are related to administrative tasks, while user
//! extrinsics are related to task operations and submissions.
//!
#![doc = simple_mermaid::mermaid!("../docs/tasks_extrinsics.mmd")]
//!
//! ## **Task Pallet Lifecycle and Operations**
//!
// #![doc = simple_mermaid::mermaid!("../docs/tasks_tb.mmd")]
//!
//! This flowchart outlines the lifecycle of a task in the task pallet. It
//! starts with task creation and checks if the shard is online. If successful,
//! the task is assigned to a shard and progresses through various phases
//! (sign, write, read). Upon completion, rewards are paid out. Tasks can also
//! be reset, and error handling includes retrying or canceling tasks.
//!
#![doc = simple_mermaid::mermaid!("../docs/tasks_lr.mmd")]
//!
//! ## **Unregister Gateways**
//! This flowchart illustrates the process for unregistering gateways in the
//! task pallet. It ensures that only a root user can perform this action. The
//! process involves clearing a specified number of gateways, all registered
//! shards, and filtering tasks to determine their status and handle them
//! appropriately. Errors during the process are logged and returned.
//!
#![doc = simple_mermaid::mermaid!("../docs/unregister_gateways.mmd")]
//!
//! ## **Reset Tasks**
//! This flowchart shows the process for resetting tasks in the task pallet. It
//! ensures that only a root user can initiate the reset. The reset process
//! includes iterating over unassigned tasks and tasks associated with
//! specific shards, resetting their phase state, and adding them back to
//! unassigned tasks if necessary. The iteration stops once the maximum
//! number of tasks to be reset is reached.
#![doc = simple_mermaid::mermaid!("../docs/reset_tasks.mmd")]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub use pallet::*;
#[cfg(test)]
mod mock;
pub mod queue;
#[cfg(test)]
mod tests;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use crate::queue::*;

	use polkadot_sdk::{
		frame_support, frame_system, pallet_balances, pallet_treasury, sp_runtime, sp_std,
	};

	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::IdentifyAccount, Saturating};
	use sp_std::boxed::Box;
	use sp_std::vec;
	use sp_std::vec::Vec;

	use time_primitives::{
		AccountId, Balance, BatchBuilder, BatchId, ErrorMsg, GatewayMessage, GatewayOp, GmpEvent,
		GmpEvents, MessageId, NetworkId, NetworksInterface, PublicKey, ShardId, ShardsInterface,
		Task, TaskId, TaskResult, TasksInterface, TssPublicKey, TssSignature,
	};

	/// Trait to define the weights for various extrinsics in the pallet.
	pub trait WeightInfo {
		fn submit_task_result() -> Weight;
		fn prepare_batches(n: u32) -> Weight;
		fn schedule_tasks(n: u32) -> Weight;
		fn submit_gmp_events() -> Weight;
		fn sync_network() -> Weight;
		fn stop_network() -> Weight;
		fn remove_task() -> Weight;
	}

	impl WeightInfo for () {
		fn submit_task_result() -> Weight {
			Weight::default()
		}
		fn prepare_batches(_: u32) -> Weight {
			Weight::default()
		}
		fn schedule_tasks(_: u32) -> Weight {
			Weight::default()
		}
		fn submit_gmp_events() -> Weight {
			Weight::default()
		}
		fn sync_network() -> Weight {
			Weight::default()
		}
		fn stop_network() -> Weight {
			Weight::default()
		}
		fn remove_task() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<AccountId = AccountId>
		+ pallet_balances::Config<Balance = Balance>
		+ pallet_treasury::Config
	{
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type WeightInfo: WeightInfo;
		type Shards: ShardsInterface;
		type Networks: NetworksInterface;
		/// Maximum number of tasks scheduled per block in `on_initialize`
		type MaxTasksPerBlock: Get<u32>;
		/// Maximum number of batches started per block in `on_initialize`
		type MaxBatchesPerBlock: Get<u32>;
	}

	/// Double map storage for unassigned tasks.
	#[pallet::storage]
	pub type UATasks<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		NetworkId,
		Blake2_128Concat,
		Index,
		TaskId,
		OptionQuery,
	>;

	/// Map storage for the insert index of unassigned tasks.
	#[pallet::storage]
	pub type UATasksInsertIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, Index, OptionQuery>;

	/// Map storage for the remove index of unassigned tasks.
	#[pallet::storage]
	pub type UATasksRemoveIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, Index, OptionQuery>;

	/// Double map storage for queued ops.
	#[pallet::storage]
	pub type Ops<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		NetworkId,
		Blake2_128Concat,
		Index,
		GatewayOp,
		OptionQuery,
	>;

	/// Map storage for the insert index of queued ops.
	#[pallet::storage]
	pub type OpsInsertIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, Index, OptionQuery>;

	/// Map storage for the remove index of queued ops.
	#[pallet::storage]
	pub type OpsRemoveIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, Index, OptionQuery>;

	/// Double map storage for tasks by shard.
	#[pallet::storage]
	pub type ShardTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TaskId, (), OptionQuery>;

	/// Map storage for task shard by task ID.
	#[pallet::storage]
	#[pallet::getter(fn task_shard)]
	pub type TaskShard<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, ShardId, OptionQuery>;

	/// Double map storage for network shards.
	#[pallet::storage]
	pub type NetworkShards<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		NetworkId,
		Blake2_128Concat,
		ShardId,
		(),
		OptionQuery,
	>;

	/// Storage for task ID counter.
	#[pallet::storage]
	pub type TaskIdCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	pub type TaskCount<T: Config> = StorageMap<_, Blake2_128Concat, NetworkId, u64, ValueQuery>;

	#[pallet::storage]
	pub type ExecutedTaskCount<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u64, ValueQuery>;

	#[pallet::storage]
	pub type ShardTaskCount<T: Config> = StorageMap<_, Blake2_128Concat, ShardId, u32, ValueQuery>;

	/// Map storage for tasks.
	#[pallet::storage]
	#[pallet::getter(fn tasks)]
	pub type Tasks<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, Task, OptionQuery>;

	#[pallet::storage]
	pub type TaskOutput<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, Result<(), ErrorMsg>, OptionQuery>;

	#[pallet::storage]
	pub type TaskNetwork<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, NetworkId, OptionQuery>;

	/// Map storage for registered shards.
	#[pallet::storage]
	pub type ShardRegistered<T: Config> =
		StorageMap<_, Blake2_128Concat, TssPublicKey, (), OptionQuery>;

	///  Map storage for received tasks.
	#[pallet::storage]
	pub type ReadEventsTask<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, TaskId, OptionQuery>;

	#[pallet::storage]
	pub type SyncHeight<T: Config> = StorageMap<_, Blake2_128Concat, NetworkId, u64, ValueQuery>;

	/// Map storage for message tasks.
	#[pallet::storage]
	pub type MessageReceivedTaskId<T: Config> =
		StorageMap<_, Blake2_128Concat, MessageId, TaskId, OptionQuery>;

	#[pallet::storage]
	pub type MessageExecutedTaskId<T: Config> =
		StorageMap<_, Blake2_128Concat, MessageId, TaskId, OptionQuery>;

	#[pallet::storage]
	pub type MessageBatchId<T: Config> =
		StorageMap<_, Blake2_128Concat, MessageId, BatchId, OptionQuery>;

	#[pallet::storage]
	pub type BatchIdCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Map storage for batches.
	#[pallet::storage]
	pub type BatchMessage<T: Config> =
		StorageMap<_, Blake2_128Concat, BatchId, GatewayMessage, OptionQuery>;

	#[pallet::storage]
	pub type BatchTaskId<T: Config> = StorageMap<_, Blake2_128Concat, BatchId, TaskId, OptionQuery>;

	/// Map storage for task signers.
	#[pallet::storage]
	pub type TaskSubmitter<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, PublicKey, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Task succeeded with optional error message
		TaskResult(TaskId, Result<(), ErrorMsg>),
		/// Set the maximum number of assigned tasks for all shards on the network
		ShardTaskLimitSet(NetworkId, u32),
		/// Set the network batch size
		BatchSizeSet(NetworkId, u64, u64),
		/// Insufficient Treasury Balance to payout rewards
		InsufficientTreasuryBalance(AccountId, Balance),
		/// Message received
		MessageReceived(MessageId),
		/// Message executed
		MessageExecuted(MessageId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Task
		UnknownTask,
		/// Unknown Shard
		UnknownShard,
		/// Invalid Signature
		InvalidSignature,
		/// Invalid task result
		InvalidTaskResult,
		/// Invalid signer
		InvalidSigner,
		/// Task not assigned
		UnassignedTask,
		/// Task already signed
		TaskSigned,
		/// Cannot submit result for GMP functions unless gateway is registered
		GatewayNotRegistered,
		/// Invalid batch id
		InvalidBatchId,
		/// Cannot remove task
		CannotRemoveTask,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			log::info!("on_initialize begin");
			let weight = Self::prepare_batches().saturating_add(Self::schedule_tasks());
			log::info!("on_initialize end");
			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Used by chroncles to submit task results.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_task_result())]
		pub fn submit_task_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			result: TaskResult,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			if TaskOutput::<T>::get(task_id).is_some() {
				return Ok(());
			}
			let shard = TaskShard::<T>::get(task_id).ok_or(Error::<T>::UnassignedTask)?;
			let network = T::Shards::shard_network(shard).ok_or(Error::<T>::UnknownShard)?;
			let reward = task.reward();
			let result = match (task, result) {
				(
					Task::ReadGatewayEvents { blocks },
					TaskResult::ReadGatewayEvents { events, signature },
				) => {
					// verify signature
					let bytes = time_primitives::encode_gmp_events(task_id, &events.0);
					Self::verify_signature(shard, &bytes, signature)?;
					// update sync height if the network wasn't manually synced
					let curr = SyncHeight::<T>::get(network);
					if curr == blocks.start {
						SyncHeight::<T>::insert(network, blocks.end);
					}
					// start next batch if network wasn't stopped
					if ReadEventsTask::<T>::get(network).is_some() {
						Self::read_gateway_events(network);
					}
					// process events
					Self::process_events(network, task_id, events);
					Ok(())
				},
				(Task::SubmitGatewayMessage { .. }, TaskResult::SubmitGatewayMessage { error }) => {
					// verify signature
					let expected_signer =
						TaskSubmitter::<T>::get(task_id).map(|s| s.into_account());
					ensure!(Some(&signer) == expected_signer.as_ref(), Error::<T>::InvalidSigner);
					Err(error)
				},
				(_, _) => return Err(Error::<T>::InvalidTaskResult.into()),
			};
			// complete task
			Self::treasury_transfer_shard(shard, reward);
			Self::finish_task(network, task_id, result);
			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_gmp_events())]
		pub fn submit_gmp_events(
			origin: OriginFor<T>,
			network: NetworkId,
			events: GmpEvents,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Self::process_events(network, 0, events);
			Ok(())
		}

		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::sync_network())]
		pub fn sync_network(
			origin: OriginFor<T>,
			network: NetworkId,
			block: u64,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			SyncHeight::<T>::insert(network, block);
			Ok(())
		}

		#[pallet::call_index(12)]
		#[pallet::weight(<T as Config>::WeightInfo::stop_network())]
		pub fn stop_network(origin: OriginFor<T>, network: NetworkId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			ReadEventsTask::<T>::remove(network);
			Ok(())
		}

		#[pallet::call_index(13)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_task())]
		pub fn remove_task(origin: OriginFor<T>, task: TaskId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			// only remove completed tasks, otherwise can cause mayhem
			if TaskOutput::<T>::take(task).is_none() {
				return Err(Error::<T>::CannotRemoveTask.into());
			}
			if let Some(Task::SubmitGatewayMessage { batch_id }) = Tasks::<T>::take(task) {
				if let Some(msg) = BatchMessage::<T>::take(batch_id) {
					for op in msg.ops {
						if let GatewayOp::SendMessage(msg) = op {
							let message = msg.message_id();
							MessageReceivedTaskId::<T>::remove(message);
							MessageBatchId::<T>::remove(message);
						}
					}
				}
				BatchTaskId::<T>::remove(batch_id);
			}
			TaskNetwork::<T>::remove(task);
			TaskSubmitter::<T>::remove(task);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn process_events(network: NetworkId, task_id: TaskId, events: GmpEvents) {
			for event in events.0 {
				match event {
					GmpEvent::ShardRegistered(pubkey) => {
						ShardRegistered::<T>::insert(pubkey, ());
					},
					GmpEvent::ShardUnregistered(pubkey) => {
						ShardRegistered::<T>::remove(pubkey);
					},
					GmpEvent::MessageReceived(msg) => {
						let msg_id = msg.message_id();
						Self::ops_queue(msg.dest_network).push(GatewayOp::SendMessage(msg));
						MessageReceivedTaskId::<T>::insert(msg_id, task_id);
						Self::deposit_event(Event::<T>::MessageReceived(msg_id));
					},
					GmpEvent::MessageExecuted(msg_id) => {
						MessageExecutedTaskId::<T>::insert(msg_id, task_id);
						Self::deposit_event(Event::<T>::MessageExecuted(msg_id));
					},
					GmpEvent::BatchExecuted(batch_id) => {
						if let Some(task_id) = BatchTaskId::<T>::get(batch_id) {
							Self::finish_task(network, task_id, Ok(()));
						}
					},
				}
			}
		}

		/// Validate a TSS (Threshold Signature Scheme) signature for data associated with a specific shard.
		///
		/// # Flow
		///   1. Retrieve the TSS public key for `shard_id`.
		///   2. Verify the `signature` against the `data` using the verifying key.
		///   3. Return `Ok(())` if verification succeeds, or an appropriate error if any step fails.
		fn verify_signature(
			shard_id: ShardId,
			data: &[u8],
			signature: TssSignature,
		) -> DispatchResult {
			let public_key = T::Shards::tss_public_key(shard_id).ok_or(Error::<T>::UnknownShard)?;
			time_primitives::verify_signature(public_key, data, signature)
				.map_err(|_| Error::<T>::InvalidSignature)?;
			Ok(())
		}

		fn treasury_transfer_shard(shard: ShardId, amount: u128) {
			let members = T::Shards::shard_members(shard);
			if members.is_empty() {
				// Handle the case where there are no members
				log::error!("Shard has no members, cannot distribute rewards.");
				return;
			}
			let member_amount = amount / members.len() as u128;
			for account in members.into_iter() {
				Self::treasury_transfer(account, member_amount);
			}
		}

		fn treasury_transfer(account: AccountId, amount: u128) {
			let treasury = pallet_treasury::Pallet::<T>::account_id();
			match pallet_balances::Pallet::<T>::transfer(
				&treasury,
				&account,
				amount,
				ExistenceRequirement::KeepAlive,
			) {
				Ok(_) => {},
				Err(err) => {
					Self::deposit_event(Event::InsufficientTreasuryBalance(account, amount));
					log::error!("Treasury transfer failed: {:?}", err);
				},
			}
		}

		pub(crate) fn create_task(network: NetworkId, task: Task) -> TaskId {
			let task_id = TaskIdCounter::<T>::get().saturating_plus_one();
			let needs_registration = task.needs_registration();
			Tasks::<T>::insert(task_id, task);
			TaskNetwork::<T>::insert(task_id, network);
			TaskIdCounter::<T>::put(task_id);
			if !needs_registration {
				ReadEventsTask::<T>::insert(network, task_id);
			} else {
				log::debug!("pallet___task adding task to unassigned queue: {:?}", task_id);
				Self::ua_task_queue(network).push(task_id);
			}
			TaskCount::<T>::insert(network, TaskCount::<T>::get(network).saturating_add(1));
			Self::deposit_event(Event::TaskCreated(task_id));
			task_id
		}

		fn finish_task(network: NetworkId, task_id: TaskId, result: Result<(), ErrorMsg>) {
			TaskOutput::<T>::insert(task_id, result.clone());
			log::debug!("pallet___task finishing task: {:?}", task_id);
			if let Some(shard) = TaskShard::<T>::take(task_id) {
				log::debug!("pallet___task Task shard found cleaning up: {:?}", task_id);
				ShardTasks::<T>::remove(shard, task_id);
				ShardTaskCount::<T>::mutate(shard, |count| {
					*count = count.saturating_sub(1);
				});
				ExecutedTaskCount::<T>::insert(
					network,
					ExecutedTaskCount::<T>::get(network).saturating_add(1),
				);
			}
			Self::deposit_event(Event::TaskResult(task_id, result));
		}

		fn read_gateway_events(network: NetworkId) -> TaskId {
			let block = SyncHeight::<T>::get(network);
			let size = T::Networks::next_batch_size(network, block) as u64;
			let end = block.saturating_add(size);
			Self::create_task(network, Task::ReadGatewayEvents { blocks: block..end })
		}

		/// Non-prioritized tasks which are assigned only after
		/// all prioritized tasks are assigned.
		fn ua_task_queue(network: NetworkId) -> Box<dyn QueueT<T, TaskId>> {
			Box::new(
				QueueImpl::<T, TaskId, UATasksInsertIndex<T>, UATasksRemoveIndex<T>, UATasks<T>>::new(
					network,
				),
			)
		}

		fn ops_queue(network: NetworkId) -> Box<dyn QueueT<T, GatewayOp>> {
			Box::new(QueueImpl::<T, GatewayOp, OpsInsertIndex<T>, OpsRemoveIndex<T>, Ops<T>>::new(
				network,
			))
		}

		pub(crate) fn is_shard_registered(shard: ShardId) -> bool {
			let Some(pubkey) = T::Shards::tss_public_key(shard) else {
				return false;
			};
			ShardRegistered::<T>::get(pubkey).is_some()
		}

		pub(crate) fn assign_task(shard: ShardId, task_id: TaskId) {
			let needs_signer =
				Tasks::<T>::get(task_id).map(|task| task.needs_signer()).unwrap_or_default();
			ShardTasks::<T>::insert(shard, task_id, ());
			TaskShard::<T>::insert(task_id, shard);
			ShardTaskCount::<T>::mutate(shard, |count| {
				*count = count.saturating_add(1);
			});
			if needs_signer {
				TaskSubmitter::<T>::insert(task_id, T::Shards::next_signer(shard));
			}
		}

		/// To schedule tasks for a specified network and optionally for a specific shard, optimizing
		/// task allocation based on current workload and system constraints.
		/// Returns number of tasks assigned
		/// # Flow
		///   1. Count the number of incomplete tasks (`tasks`) for the specified `shard_id`.
		///   2. Determine the size of the shard (`shard_size`) based on the number of shard members.
		///   3. Check if the `shard_id` is registered.
		///   4. Retrieve the maximum allowed tasks (`shard_task_limit`) for the network, defaulting to 10 if unspecified.
		///   5. Calculate the remaining capacity (`capacity`) for new tasks.
		///   6. If `capacity` is zero, stop further task assignments.
		///   7. Get system tasks and, if space permits, non-system tasks.
		///   8. Assign each task to the shard using `Self::assign_task(network, shard_id, index, task)`.
		fn schedule_tasks_shard(network: NetworkId, shard_id: ShardId, capacity: u32) -> u32 {
			let mut num_tasks_assigned = 0u32;
			let queue = Self::ua_task_queue(network);
			log::debug!("pallet___tasks capacity: {:?}", capacity);
			for _ in 0..capacity {
				let Some(task) = queue.pop() else {
					break;
				};
				log::debug!("pallet___tasks assigned task: {:?}", task);
				Self::assign_task(shard_id, task);
				num_tasks_assigned = num_tasks_assigned.saturating_plus_one();
			}
			num_tasks_assigned
		}

		/// Schedule tasks for a specified network, optionally targeting a specific shard if provided.
		///
		/// # Flow
		/// for network in networks:
		/// 	tasks_per_shard = (assigned_tasks(network) + unassigned_tasks(network)) / number_of_registered_shards(network)
		/// 	tasks_per_shard = min(tasks_per_shard, max_assignable_tasks)
		/// 	for registered_shard in network:
		/// 		number_of_tasks_to_assign = min(tasks_per_shard, shard_capacity(registered_shard))
		pub(crate) fn schedule_tasks() -> Weight {
			let mut num_tasks_assigned: u32 = 0u32;
			for (network, task_id) in ReadEventsTask::<T>::iter() {
				let max_assignable_tasks = T::Networks::shard_task_limit(network);
				// log::debug!("max assignable tasks are: {:?}", max_assignable_tasks);

				// handle read events task assignment
				if TaskShard::<T>::get(task_id).is_none() {
					for (shard, _) in NetworkShards::<T>::iter_prefix(network) {
						if ShardTaskCount::<T>::get(shard) < max_assignable_tasks {
							if num_tasks_assigned == T::MaxTasksPerBlock::get() {
								return <T as Config>::WeightInfo::schedule_tasks(
									T::MaxTasksPerBlock::get(),
								);
							}
							Self::assign_task(shard, task_id);
							num_tasks_assigned = num_tasks_assigned.saturating_plus_one();
							break;
						}
					}
				}

				// collect registered shards
				let registered_shards: Vec<ShardId> = NetworkShards::<T>::iter_prefix(network)
					.map(|(shard, _)| shard)
					.filter(|shard| Self::is_shard_registered(*shard))
					.collect();
				if registered_shards.is_empty() {
					continue;
				}

				// calculate tasks per shard
				let task_count = TaskCount::<T>::get(network);
				log::debug!("pallet___tasks schedule...task_count: {:?}", task_count);
				let executed_task_count = ExecutedTaskCount::<T>::get(network);
				log::debug!(
					"pallet___tasks schedule...executed_task_count: {:?}",
					executed_task_count
				);
				let assignable_task_count = task_count - executed_task_count;
				log::debug!(
					"pallet___tasks schedule...assignable_task_count: {:?}",
					assignable_task_count
				);
				// Problem no 1:
				// these assignable task count could contain tasks which are already assigned to other shard and is in process of executing.
				let tasks_per_shard = assignable_task_count as u32 / registered_shards.len() as u32;
				log::debug!("pallet___tasks schedule...task_per_shard: {:?}", tasks_per_shard);
				let tasks_per_shard = core::cmp::min(tasks_per_shard, max_assignable_tasks);
				log::debug!("pallet___tasks schedule...task_per_shard: {:?}", tasks_per_shard);
				for (_, task_id) in UATasks::<T>::iter_prefix(network) {
					log::debug!("pallet___tasks schedule...ua tasks: {:?}", task_id);
				}

				// assign tasks
				for shard in registered_shards {
					let shard_task_count = ShardTaskCount::<T>::get(shard);
					let capacity = tasks_per_shard.saturating_sub(shard_task_count);
					log::debug!(
						"pallet___tasks schedule... capacity: {:?}, task_per_shard: {:?}, shard_task_count: {:?}",
						capacity,
						tasks_per_shard,
						shard_task_count
					);
					if T::MaxTasksPerBlock::get() > num_tasks_assigned.saturating_add(capacity) {
						num_tasks_assigned = num_tasks_assigned
							.saturating_add(Self::schedule_tasks_shard(network, shard, capacity));
					} else {
						Self::schedule_tasks_shard(
							network,
							shard,
							T::MaxTasksPerBlock::get().saturating_sub(num_tasks_assigned),
						);
						return <T as Config>::WeightInfo::schedule_tasks(
							T::MaxTasksPerBlock::get(),
						);
					}
				}
			}
			<T as Config>::WeightInfo::schedule_tasks(num_tasks_assigned)
		}

		pub(crate) fn prepare_batches() -> Weight {
			let mut num_batches_started = 0u32;
			for (network, _) in ReadEventsTask::<T>::iter() {
				let batch_gas_limit = T::Networks::batch_gas_limit(network);
				let mut batcher = BatchBuilder::new(batch_gas_limit);
				let queue = Self::ops_queue(network);
				while let Some(op) = queue.pop() {
					if let Some(msg) = batcher.push(op) {
						if num_batches_started == T::MaxBatchesPerBlock::get() {
							return <T as Config>::WeightInfo::prepare_batches(
								T::MaxBatchesPerBlock::get(),
							);
						}
						Self::start_batch(network, msg);
						num_batches_started = num_batches_started.saturating_plus_one();
					}
				}
				if num_batches_started == T::MaxBatchesPerBlock::get() {
					return <T as Config>::WeightInfo::prepare_batches(T::MaxBatchesPerBlock::get());
				}
				if let Some(msg) = batcher.take_batch() {
					Self::start_batch(network, msg);
					num_batches_started = num_batches_started.saturating_plus_one();
				}
			}
			<T as Config>::WeightInfo::prepare_batches(num_batches_started)
		}

		fn start_batch(network: NetworkId, msg: GatewayMessage) {
			let batch_id = BatchIdCounter::<T>::get();
			BatchIdCounter::<T>::put(batch_id.saturating_add(1));
			for op in &msg.ops {
				if let GatewayOp::SendMessage(msg) = op {
					let msg_id = msg.message_id();
					MessageBatchId::<T>::insert(msg_id, batch_id);
				}
			}
			BatchMessage::<T>::insert(batch_id, msg);
			let task_id = Self::create_task(network, Task::SubmitGatewayMessage { batch_id });
			BatchTaskId::<T>::insert(batch_id, task_id);
		}
	}

	impl<T: Config> Pallet<T> {
		/// Retrieves the public key of the signer for a given task.
		/// Look up the `PublicKey` of the signer associated with the provided `task` ID in the storage.
		pub fn get_task_submitter(task: TaskId) -> Option<PublicKey> {
			TaskSubmitter::<T>::get(task)
		}

		/// Retrieves a list of tasks associated with a given shard.
		/// Look up the tasks associated with the provided `shard_id` in the storage.
		pub fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskId> {
			ShardTasks::<T>::iter_prefix(shard_id).map(|(task_id, _)| task_id).collect()
		}

		/// Retrieves the descriptor for a given task.
		/// Look up the `TaskDescriptor` associated with the provided `task_id` in the storage.
		pub fn get_task(task_id: TaskId) -> Option<Task> {
			Tasks::<T>::get(task_id)
		}

		/// Retrieves the shard ID associated with a given task.
		/// Look up the shard ID associated with the provided `task_id` in the storage.
		pub fn get_task_shard(task_id: TaskId) -> Option<ShardId> {
			TaskShard::<T>::get(task_id)
		}

		/// Retrieves the result of a given task.
		/// Look up the `TaskResult` associated with the provided `task_id` in the storage.
		pub fn get_task_result(task_id: TaskId) -> Option<Result<(), ErrorMsg>> {
			TaskOutput::<T>::get(task_id)
		}

		pub fn get_batch_message(batch: BatchId) -> Option<GatewayMessage> {
			BatchMessage::<T>::get(batch)
		}
	}

	impl<T: Config> TasksInterface for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: NetworkId) {
			NetworkShards::<T>::insert(network, shard_id, ());
			if T::Networks::gateway(network).is_some() {
				let Some(key) = T::Shards::tss_public_key(shard_id) else {
					return;
				};
				Self::ops_queue(network).push(GatewayOp::RegisterShard(key));
			}
		}

		fn shard_offline(shard_id: ShardId, network: NetworkId) {
			NetworkShards::<T>::remove(network, shard_id);
			// unassign tasks
			ShardTasks::<T>::drain_prefix(shard_id).for_each(|(task_id, _)| {
				TaskShard::<T>::remove(task_id);
				Self::ua_task_queue(network).push(task_id);
			});
			ShardTaskCount::<T>::insert(shard_id, 0);
			let Some(key) = T::Shards::tss_public_key(shard_id) else {
				return;
			};
			Self::ops_queue(network).push(GatewayOp::UnregisterShard(key));
		}

		fn gateway_registered(network: NetworkId, block: u64) {
			SyncHeight::<T>::insert(network, block);
			Self::read_gateway_events(network);
		}
	}
}
