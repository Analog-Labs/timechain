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
	use scale_info::prelude::string::String;

	use polkadot_sdk::{
		frame_support, frame_system, pallet_balances, pallet_treasury, sp_runtime, sp_std,
	};

	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::IdentifyAccount, Saturating};
	use sp_std::boxed::Box;
	use sp_std::vec;
	use sp_std::vec::Vec;

	use time_primitives::{
		AccountId, Balance, BatchId, ElectionsInterface, GatewayMessage, GatewayOp, GmpEvent,
		GmpParams, MessageId, NetworkId, NetworksInterface, PublicKey, ShardId, ShardsInterface,
		Task, TaskId, TaskResult, TasksInterface, TransferStake, TssPublicKey, TssSignature,
	};

	/// Trait to define the weights for various extrinsics in the pallet.
	pub trait WeightInfo {
		fn submit_task_result() -> Weight;
		fn set_shard_task_limit() -> Weight;
	}

	impl WeightInfo for () {
		fn submit_task_result() -> Weight {
			Weight::default()
		}

		fn set_shard_task_limit() -> Weight {
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
		type Elections: ElectionsInterface;
		type Members: TransferStake;
		type Networks: NetworksInterface;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
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

	/// Double map storage for unassigned tasks.
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

	/// Map storage for the insert index of unassigned tasks.
	#[pallet::storage]
	pub type OpsInsertIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, Index, OptionQuery>;

	/// Map storage for the remove index of unassigned tasks.
	#[pallet::storage]
	pub type OpsRemoveIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, Index, OptionQuery>;

	/// Map storage for shard task limits.
	#[pallet::storage]
	#[pallet::getter(fn shard_task_limit)]
	pub type ShardTaskLimit<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u32, OptionQuery>;

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

	/// Map storage for tasks.
	#[pallet::storage]
	#[pallet::getter(fn tasks)]
	pub type Tasks<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, Task, OptionQuery>;

	#[pallet::storage]
	pub type TaskOutput<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, Result<(), String>, OptionQuery>;

	/// Map storage for registered shards.
	#[pallet::storage]
	pub type ShardRegistered<T: Config> =
		StorageMap<_, Blake2_128Concat, TssPublicKey, (), OptionQuery>;

	///  Map storage for received tasks.
	#[pallet::storage]
	pub type ReadEventsTask<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u64, OptionQuery>;

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

	/// Map storage for batches.
	#[pallet::storage]
	pub type BatchMessage<T: Config> =
		StorageMap<_, Blake2_128Concat, BatchId, GatewayMessage, OptionQuery>;

	/// Map storage for batch signatures
	#[pallet::storage]
	pub type BatchSignature<T: Config> =
		StorageMap<_, Blake2_128Concat, BatchId, TssSignature, OptionQuery>;

	/// Map storage for task signers.
	#[pallet::storage]
	pub type TaskSigner<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, PublicKey, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Task succeeded with optional error message
		TaskResult(TaskId, Result<(), String>),
		/// Set the maximum number of assigned tasks for all shards on the network
		ShardTaskLimitSet(NetworkId, u32),
		/// Set the network batch size
		BatchSizeSet(NetworkId, u64, u64),
		/// Insufficient Treasury Balance to payout rewards
		InsufficientTreasuryBalance(TaskId),
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
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			Self::schedule_tasks()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Used by chroncles to submit task results.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_task_result())]
		pub fn submit_result(
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
			let gateway = T::Networks::gateway(network).ok_or(Error::<T>::GatewayNotRegistered)?;
			let reward = task.reward();
			let task_result = match (task, result) {
				(
					Task::ReadGatewayEvents { blocks },
					TaskResult::ReadGatewayEvents { events, signature },
				) => {
					// verify signature
					let bytes = time_primitives::encode_gmp_events(task_id, &events);
					Self::verify_signature(shard, &bytes, signature)?;
					// start next batch
					let start = blocks.end;
					let size = T::Networks::next_batch_size(network, start);
					let end = start + size;
					Self::create_task(network, Task::ReadGatewayEvents { blocks: start..end });
					// process events
					for event in events {
						match event {
							GmpEvent::ShardRegistered(pubkey) => {
								ShardRegistered::<T>::insert(pubkey, ());
							},
							GmpEvent::ShardUnregistered(pubkey) => {
								ShardRegistered::<T>::remove(pubkey);
							},
							GmpEvent::MessageReceived(msg) => {
								let msg_id = msg.message_id();
								Self::ops_queue(network).push(GatewayOp::SendMessage(msg));
								MessageReceivedTaskId::<T>::insert(msg_id, task_id);
								Self::deposit_event(Event::<T>::MessageReceived(msg_id));
							},
							GmpEvent::MessageExecuted(msg_id) => {
								MessageExecutedTaskId::<T>::insert(msg_id, task_id);
								Self::deposit_event(Event::<T>::MessageExecuted(msg_id));
							},
						}
					}
					// transfer reward
					Self::treasury_transfer_shard(shard, reward);
					Ok(())
				},
				(
					Task::SignGatewayMessage { batch_id },
					TaskResult::SignGatewayMessage { signature },
				) => {
					// verify signature
					let msg = BatchMessage::<T>::get(batch_id).ok_or(Error::<T>::InvalidBatchId)?;
					let payload = msg.encode();
					let params = GmpParams { network, gateway };
					let bytes = params.hash(&payload);
					Self::verify_signature(shard, &bytes, signature)?;
					// store signature
					BatchSignature::<T>::insert(batch_id, signature);
					// start submission
					Self::create_task(network, Task::SubmitGatewayMessage { batch_id });
					// transfer reward
					Self::treasury_transfer_shard(shard, reward);
					// result
					Ok(())
				},
				(Task::SubmitGatewayMessage { .. }, TaskResult::SubmitGatewayMessage { error }) => {
					// verify signature
					let expected_signer = TaskSigner::<T>::get(task_id).map(|s| s.into_account());
					ensure!(Some(&signer) == expected_signer.as_ref(), Error::<T>::InvalidSigner);
					// transfer reward
					Self::treasury_transfer(signer, reward);
					// result
					if let Some(error) = error {
						Err(error)
					} else {
						Ok(())
					}
				},
				(_, _) => return Err(Error::<T>::InvalidTaskResult.into()),
			};
			TaskOutput::<T>::insert(task_id, task_result.clone());
			TaskShard::<T>::take(task_id);
			ShardTasks::<T>::remove(shard, task_id);
			Self::deposit_event(Event::TaskResult(task_id, task_result));
			Ok(())
		}

		///  Sets the task limit for a specific shard.
		/// # Flow
		///    1. Ensure the origin of the transaction is a root user.
		///    2. Insert the new task limit for the specified network into the [`ShardTaskLimit`] storage.
		///    3. Emit an event indicating the shard task limit has been set.
		///    4. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::set_shard_task_limit())]
		pub fn set_shard_task_limit(
			origin: OriginFor<T>,
			network: NetworkId,
			limit: u32,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			ShardTaskLimit::<T>::insert(network, limit);
			Self::deposit_event(Event::ShardTaskLimitSet(network, limit));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Validate a TSS (Threshold Signature Scheme) signature for data associated with a specific shard.
		///
		/// # Flow
		///   1. Retrieve the TSS public key for `shard_id`.
		///   2. Convert the provided `signature` into a [`schnorr_evm::Signature`].
		///   3. Convert the retrieved public key into a [`schnorr_evm::VerifyingKey`].
		///   4. Verify the `signature` against the `data` using the verifying key.
		///   5. Return `Ok(())` if verification succeeds, or an appropriate error if any step fails.
		fn verify_signature(
			shard_id: ShardId,
			data: &[u8],
			signature: TssSignature,
		) -> DispatchResult {
			let public_key = T::Shards::tss_public_key(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let signature = schnorr_evm::Signature::from_bytes(signature)
				.map_err(|_| Error::<T>::InvalidSignature)?;
			let schnorr_public_key = schnorr_evm::VerifyingKey::from_bytes(public_key)
				.map_err(|_| Error::<T>::UnknownShard)?;
			schnorr_public_key
				.verify(data, &signature)
				.map_err(|_| Error::<T>::InvalidSignature)?;
			Ok(())
		}

		fn treasury_transfer_shard(shard: ShardId, amount: u128) {
			let members = T::Shards::shard_members(shard);
			let member_amount = amount / members.len() as u128;
			for account in members.into_iter() {
				Self::treasury_transfer(account, member_amount);
			}
		}

		fn treasury_transfer(account: AccountId, amount: u128) {
			let treasury = pallet_treasury::Pallet::<T>::account_id();
			let _ = pallet_balances::Pallet::<T>::transfer(
				&treasury,
				&account,
				amount,
				ExistenceRequirement::KeepAlive,
			);
		}

		fn create_task(network: NetworkId, task: Task) -> TaskId {
			let task_id = TaskIdCounter::<T>::get();
			Tasks::<T>::insert(task_id, task);
			TaskIdCounter::<T>::put(task_id.saturating_plus_one());
			Self::ua_task_queue(network).push(task_id);
			Self::deposit_event(Event::TaskCreated(task_id));
			task_id
		}

		/// Non-prioritized tasks which are assigned only after
		/// all prioritized tasks are assigned.
		fn ua_task_queue(network: NetworkId) -> Box<dyn QueueT<T, TaskId>> {
			Box::new(
				QueueImpl::<TaskId, UATasksInsertIndex<T>, UATasksRemoveIndex<T>, UATasks<T>>::new(
					network,
				),
			)
		}

		fn ops_queue(network: NetworkId) -> Box<dyn QueueT<T, GatewayOp>> {
			Box::new(QueueImpl::<GatewayOp, OpsInsertIndex<T>, OpsRemoveIndex<T>, Ops<T>>::new(
				network,
			))
		}

		fn is_shard_registered(shard: ShardId) -> bool {
			let Some(pubkey) = T::Shards::tss_public_key(shard) else {
				return false;
			};
			ShardRegistered::<T>::get(pubkey).is_some()
		}

		fn assign_task(
			network: NetworkId,
			shard: ShardId,
			task_id: TaskId,
			task_index: Index,
		) -> Weight {
			let (mut reads, mut writes) = (0, 0);
			let needs_signer =
				Tasks::<T>::get(task_id).map(|task| task.needs_signer()).unwrap_or_default();
			if let Some(old_shard_id) = TaskShard::<T>::get(task_id) {
				ShardTasks::<T>::remove(old_shard_id, task_id);
				// writes: ShardTasks
				writes = writes.saturating_plus_one();
			}
			Self::ua_task_queue(network).remove(task_index);
			ShardTasks::<T>::insert(shard, task_id, ());
			TaskShard::<T>::insert(task_id, shard);
			if needs_signer {
				TaskSigner::<T>::insert(task_id, T::Shards::next_signer(shard));
				writes = writes.saturating_plus_one();
			}
			// writes: remove_unassigned_task, ShardTasks, TaskShard, start_phase
			writes = writes.saturating_add(4);
			// reads: TaskShard, TaskPhaseState
			reads = reads.saturating_add(2);
			T::DbWeight::get()
				.reads(reads)
				.saturating_add(T::DbWeight::get().writes(writes))
		}

		/// To schedule tasks for a specified network and optionally for a specific shard, optimizing
		/// task allocation based on current workload and system constraints.
		///
		/// # Flow
		///   1. Count the number of incomplete tasks (`tasks`) for the specified `shard_id`.
		///   2. Determine the size of the shard (`shard_size`) based on the number of shard members.
		///   3. Check if the `shard_id` is registered.
		///   4. Retrieve the maximum allowed tasks (`shard_task_limit`) for the network, defaulting to 10 if unspecified.
		///   5. Calculate the remaining capacity (`capacity`) for new tasks.
		///   6. If `capacity` is zero, stop further task assignments.
		///   7. Get system tasks and, if space permits, non-system tasks.
		///   8. Assign each task to the shard using `Self::assign_task(network, shard_id, index, task)`.
		fn schedule_tasks_shard(network: NetworkId, shard_id: ShardId, capacity: usize) -> Weight {
			let mut reads = 0;
			let tasks = Self::ua_task_queue(network).get_n(capacity);
			// reads: T::Shards::shard_members, ShardRegistered, prioritized_unassigned_tasks
			reads = reads.saturating_add(3);
			let mut weight = T::DbWeight::get().reads(reads);
			for (index, task) in tasks {
				weight = weight.saturating_add(Self::assign_task(network, shard_id, index, task));
			}
			weight
		}

		/// Schedule tasks for a specified network, optionally targeting a specific shard if provided.
		///
		/// # Flow
		/// for network in networks:
		/// 	tasks_per_shard = (assigned_tasks(network) + unassigned_tasks(network)) / number_of_registered_shards(network)
		/// 	tasks_per_shard = min(tasks_per_shard, max_assignable_tasks)
		/// 	for registered_shard in network:
		/// 		number_of_tasks_to_assign = min(tasks_per_shard, shard_capacity(registered_shard))
		fn schedule_tasks() -> Weight {
			const DEFAULT_SHARD_TASK_LIMIT: u32 = 10;
			// To account for any computation involved outside of accounted reads/writes
			// Overestimating can lead to more consistent block times especially if weight was underestimated prior to adding the safety margin
			const WEIGHT_SAFETY_MARGIN: Weight = Weight::from_parts(500_000_000, 0);
			let mut weight = Weight::default();
			for network in 0..T::Networks::max_network_id() {
				// for this network, compute unassigned tasks count for this network
				let unassigned_task_count = UATasks::<T>::iter_prefix(network).count();
				// READs: Gateway, UnassignedTasks, UnassignedSystemTasks
				weight = weight.saturating_add(T::DbWeight::get().reads(3));
				// for this network, compute assigned task count and registered shards
				let (assigned_tasks_count, mut registered_shards) = (0usize, Vec::new());
				for (shard, _) in NetworkShards::<T>::iter_prefix(network) {
					if Self::is_shard_registered(shard) {
						registered_shards.push(shard);
					}
					// READs: NetworkShards, ShardRegistered, UnassignedSystemTasks
					weight = weight.saturating_add(T::DbWeight::get().reads(3));
				}
				// for this network, assign 0 tasks if 0 registered shards
				if registered_shards.is_empty() {
					continue;
				}
				// for this network, compute a number of tasks per shard to balance task allocation
				let mut tasks_per_shard = (assigned_tasks_count
					.saturating_add(unassigned_task_count))
				.saturating_div(registered_shards.len());
				let max_assignable_tasks =
					ShardTaskLimit::<T>::get(network).unwrap_or(DEFAULT_SHARD_TASK_LIMIT) as usize;
				tasks_per_shard = sp_std::cmp::min(tasks_per_shard, max_assignable_tasks);
				// READs: ShardTaskLimit
				weight = weight.saturating_add(T::DbWeight::get().reads(1));
				// for this network, assign unassigned tasks to registered shards evenly
				for shard in registered_shards {
					let shard_capacity = max_assignable_tasks
						.saturating_sub(ShardTasks::<T>::iter_prefix(shard).count());
					let tasks_for_shard = sp_std::cmp::min(tasks_per_shard, shard_capacity);
					weight = weight
						.saturating_add(Self::schedule_tasks_shard(network, shard, tasks_for_shard))
						// READS: ShardTasks
						.saturating_add(T::DbWeight::get().reads(1));
				}
			}
			weight.saturating_add(WEIGHT_SAFETY_MARGIN)
		}
	}

	impl<T: Config> Pallet<T> {
		/// Retrieves the public key of the signer for a given task.
		/// Look up the `PublicKey` of the signer associated with the provided `task` ID in the storage.
		pub fn get_task_signer(task: TaskId) -> Option<PublicKey> {
			TaskSigner::<T>::get(task)
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
		pub fn get_task_result(task_id: TaskId) -> Option<Result<(), String>> {
			TaskOutput::<T>::get(task_id)
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
			let Some(key) = T::Shards::tss_public_key(shard_id) else {
				return;
			};
			Self::ops_queue(network).push(GatewayOp::UnregisterShard(key));
		}

		fn gateway_registered(network: NetworkId, block: u64) {
			let size = T::Networks::next_batch_size(network, block);
			let end = block + size;
			Self::create_task(network, Task::ReadGatewayEvents { blocks: block..end });
		}
	}
}
