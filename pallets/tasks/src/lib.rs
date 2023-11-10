#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub use pallet::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::IdentifyAccount, Saturating};
	use sp_std::vec;
	use sp_std::vec::Vec;
	use time_primitives::{
		AccountId, Function, Network, ShardId, ShardsInterface, TaskCycle, TaskDescriptor,
		TaskDescriptorParams, TaskError, TaskExecution, TaskId, TaskPhase, TaskResult, TaskStatus,
		TasksInterface, TssSignature,
	};

	pub trait WeightInfo {
		fn create_task() -> Weight;
		fn stop_task() -> Weight;
		fn resume_task() -> Weight;
		fn submit_result() -> Weight;
		fn submit_error() -> Weight;
		fn submit_hash() -> Weight;
		fn submit_signature() -> Weight;
	}

	impl WeightInfo for () {
		fn create_task() -> Weight {
			Weight::default()
		}

		fn stop_task() -> Weight {
			Weight::default()
		}

		fn resume_task() -> Weight {
			Weight::default()
		}

		fn submit_result() -> Weight {
			Weight::default()
		}

		fn submit_error() -> Weight {
			Weight::default()
		}

		fn submit_hash() -> Weight {
			Weight::default()
		}

		fn submit_signature() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Shards: ShardsInterface;
		#[pallet::constant]
		type MaxRetryCount: Get<u8>;
		#[pallet::constant]
		type WritePhaseTimeout: Get<BlockNumberFor<Self>>;
	}

	#[pallet::storage]
	pub type UnassignedTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	pub type ShardTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_shard)]
	pub type TaskShard<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, ShardId, OptionQuery>;

	#[pallet::storage]
	pub type NetworkShards<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, ShardId, (), OptionQuery>;

	#[pallet::storage]
	pub type TaskIdCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tasks)]
	pub type Tasks<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskDescriptor, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_state)]
	pub type TaskState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskStatus, OptionQuery>;

	#[pallet::storage]
	pub type TaskPhaseState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskPhase, ValueQuery>;

	#[pallet::storage]
	pub type TaskSignature<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TssSignature, OptionQuery>;

	#[pallet::storage]
	pub type WritePhaseStart<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	pub type TaskRetryCounter<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, u8, ValueQuery>;

	#[pallet::storage]
	pub type TaskCycleState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskCycle, ValueQuery>;

	#[pallet::storage]
	pub type TaskResults<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TaskId,
		Blake2_128Concat,
		TaskCycle,
		TaskResult,
		OptionQuery,
	>;

	#[pallet::storage]
	pub type RegisterShardScheduled<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, (), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Updated cycle status
		TaskResult(TaskId, TaskCycle, TaskResult),
		/// Task failed due to more errors than max retry count
		TaskFailed(TaskId, TaskCycle, TaskError),
		/// Task stopped by owner
		TaskStopped(TaskId),
		/// Task resumed by owner
		TaskResumed(TaskId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Task
		UnknownTask,
		/// Unknown Shard
		UnknownShard,
		/// Invalid Signature
		InvalidSignature,
		/// Invalid Task State
		InvalidTaskState,
		/// Invalid Owner
		InvalidOwner,
		/// Invalid cycle
		InvalidCycle,
		/// Cycle must be greater than zero
		CycleMustBeGreaterThanZero,
		/// Not sign phase
		NotSignPhase,
		/// Not write phase
		NotWritePhase,
		/// Invalid signer
		InvalidSigner,
		/// Task not assigned
		UnassignedTask,
		/// Task already signed
		TaskSigned,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_task())]
		pub fn create_task(origin: OriginFor<T>, schedule: TaskDescriptorParams) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(schedule.cycle > 0, Error::<T>::CycleMustBeGreaterThanZero);
			let task_id = TaskIdCounter::<T>::get();
			if matches!(schedule.function, Function::SendMessage { .. }) {
				TaskPhaseState::<T>::insert(task_id, TaskPhase::Sign);
			}
			Tasks::<T>::insert(
				task_id,
				TaskDescriptor {
					owner: who,
					network: schedule.network,
					function: schedule.function,
					cycle: schedule.cycle,
					start: schedule.start,
					period: schedule.period,
					hash: schedule.hash,
				},
			);
			TaskState::<T>::insert(task_id, TaskStatus::Created);
			TaskIdCounter::<T>::put(task_id.saturating_plus_one());
			UnassignedTasks::<T>::insert(schedule.network, task_id, ());
			Self::deposit_event(Event::TaskCreated(task_id));
			Self::schedule_tasks(schedule.network);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::stop_task())]
		pub fn stop_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
			Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(Self::try_root_or_owner(origin, task_id), Error::<T>::InvalidOwner);
			ensure!(
				TaskState::<T>::get(task_id) == Some(TaskStatus::Created),
				Error::<T>::InvalidTaskState
			);
			TaskState::<T>::insert(task_id, TaskStatus::Stopped);
			Self::deposit_event(Event::TaskStopped(task_id));
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::resume_task())]
		pub fn resume_task(origin: OriginFor<T>, task_id: TaskId, start: u64) -> DispatchResult {
			let mut task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(Self::try_root_or_owner(origin, task_id), Error::<T>::InvalidOwner);
			ensure!(Self::is_resumable(task_id), Error::<T>::InvalidTaskState);
			task.start = start;
			Tasks::<T>::insert(task_id, task);
			TaskState::<T>::insert(task_id, TaskStatus::Created);
			TaskRetryCounter::<T>::insert(task_id, 0);
			Self::deposit_event(Event::TaskResumed(task_id));
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::submit_result())]
		pub fn submit_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			status: TaskResult,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			if TaskResults::<T>::get(task_id, cycle).is_some() {
				return Ok(());
			}
			Self::validate_signature(status.shard_id, status.hash, status.signature)?;
			ensure!(TaskCycleState::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			TaskCycleState::<T>::insert(task_id, cycle.saturating_plus_one());
			TaskResults::<T>::insert(task_id, cycle, status.clone());
			TaskRetryCounter::<T>::insert(task_id, 0);
			if Self::is_complete(task_id) {
				if let Some(shard_id) = TaskShard::<T>::take(task_id) {
					ShardTasks::<T>::remove(shard_id, task_id);
				}
				TaskState::<T>::insert(task_id, TaskStatus::Completed);
			}
			Self::deposit_event(Event::TaskResult(task_id, cycle, status));
			Ok(())
		}

		/// Submit Task Error
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::submit_error())]
		pub fn submit_error(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			error: TaskError,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			Self::validate_signature(
				error.shard_id,
				schnorr_evm::VerifyingKey::message_hash(error.msg.as_bytes()),
				error.signature,
			)?;
			ensure!(TaskCycleState::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			let retry_count = TaskRetryCounter::<T>::get(task_id);
			let new_retry_count = retry_count.saturating_plus_one();
			TaskRetryCounter::<T>::insert(task_id, new_retry_count);
			// task fails when new retry count == max - 1 => old retry count == max
			if new_retry_count == T::MaxRetryCount::get() {
				TaskState::<T>::insert(task_id, TaskStatus::Failed { error: error.clone() });
				Self::deposit_event(Event::TaskFailed(task_id, cycle, error));
			}
			Ok(())
		}

		/// Submit Task Hash
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::submit_hash())]
		pub fn submit_hash(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			hash: Vec<u8>,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			ensure!(TaskCycleState::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			let TaskPhase::Write(public_key) = TaskPhaseState::<T>::get(task_id) else {
				return Err(Error::<T>::NotWritePhase.into());
			};
			ensure!(signer == public_key.into_account(), Error::<T>::InvalidSigner);
			WritePhaseStart::<T>::remove(task_id);
			TaskPhaseState::<T>::insert(task_id, TaskPhase::Read(Some(hash)));
			Ok(())
		}

		/// Submit Signature
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::submit_signature())] // TODO update bench, weights
		pub fn submit_signature(
			origin: OriginFor<T>,
			task_id: TaskId,
			signature: TssSignature,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(TaskSignature::<T>::get(task_id).is_none(), Error::<T>::TaskSigned);
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			let TaskPhase::Sign = TaskPhaseState::<T>::get(task_id) else {
				return Err(Error::<T>::NotSignPhase.into());
			};
			let Some(shard_id) = TaskShard::<T>::get(task_id) else {
				return Err(Error::<T>::UnassignedTask.into());
			};
			Self::start_write_phase(task_id, shard_id);
			TaskSignature::<T>::insert(task_id, signature);
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut writes = 0;
			WritePhaseStart::<T>::iter().for_each(|(task_id, created_block)| {
				if n.saturating_sub(created_block) >= T::WritePhaseTimeout::get() {
					if let Some(shard_id) = TaskShard::<T>::get(task_id) {
						Self::start_write_phase(task_id, shard_id);
						writes += 2;
					}
				}
			});
			T::DbWeight::get().writes(writes)
		}
	}

	impl<T: Config> Pallet<T> {
		fn try_root_or_owner(origin: OriginFor<T>, task_id: TaskId) -> bool {
			ensure_root(origin.clone()).is_ok()
				|| Tasks::<T>::get(task_id).map_or(false, |task| {
					if let Ok(origin) = ensure_signed(origin) {
						task.owner == origin
					} else {
						false
					}
				})
		}

		fn start_write_phase(task_id: TaskId, shard_id: ShardId) {
			TaskPhaseState::<T>::insert(
				task_id,
				TaskPhase::Write(T::Shards::random_signer(shard_id)),
			);
			WritePhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number());
		}

		pub fn get_task_signature(task: TaskId) -> Option<TssSignature> {
			TaskSignature::<T>::get(task)
		}

		pub fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution> {
			ShardTasks::<T>::iter_prefix(shard_id)
				.filter(|(task_id, _)| Self::is_runnable(*task_id))
				.map(|(task_id, _)| {
					TaskExecution::new(
						task_id,
						TaskCycleState::<T>::get(task_id),
						TaskRetryCounter::<T>::get(task_id),
						TaskPhaseState::<T>::get(task_id),
					)
				})
				.collect()
		}

		pub fn get_task(task_id: TaskId) -> Option<TaskDescriptor> {
			Tasks::<T>::get(task_id)
		}

		fn is_complete(task_id: TaskId) -> bool {
			if let Some(task) = Tasks::<T>::get(task_id) {
				TaskResults::<T>::contains_key(task_id, task.cycle.saturating_less_one())
			} else {
				true
			}
		}

		fn is_resumable(task_id: TaskId) -> bool {
			matches!(
				TaskState::<T>::get(task_id),
				Some(TaskStatus::Stopped) | Some(TaskStatus::Failed { .. })
			)
		}

		fn is_runnable(task_id: TaskId) -> bool {
			matches!(TaskState::<T>::get(task_id), Some(TaskStatus::Created))
		}

		fn is_payable(task_id: TaskId) -> bool {
			let Some(task) = Self::get_task(task_id) else {
				return false;
			};
			task.function.is_payable()
		}

		fn validate_signature(
			shard_id: ShardId,
			hash: [u8; 32],
			signature: TssSignature,
		) -> DispatchResult {
			let public_key = T::Shards::tss_public_key(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let signature = schnorr_evm::Signature::from_bytes(signature)
				.map_err(|_| Error::<T>::InvalidSignature)?;
			let schnorr_public_key = schnorr_evm::VerifyingKey::from_bytes(public_key)
				.map_err(|_| Error::<T>::UnknownShard)?;
			schnorr_public_key
				.verify_prehashed(hash, &signature)
				.map_err(|_| Error::<T>::InvalidSignature)?;
			Ok(())
		}

		fn shard_task_count(shard_id: ShardId) -> usize {
			ShardTasks::<T>::iter_prefix(shard_id).count()
		}

		fn task_assignable_to_shard(task: TaskId, shard: ShardId) -> bool {
			if matches!(TaskPhaseState::<T>::get(task), TaskPhase::Write(_)) {
				T::Shards::is_shard_registered(shard) && T::Shards::is_shard_online(shard)
			} else {
				T::Shards::is_shard_online(shard)
			}
		}

		fn schedule_tasks(network: Network) {
			for (task_id, _) in UnassignedTasks::<T>::iter_prefix(network) {
				let shard = NetworkShards::<T>::iter_prefix(network)
					.filter(|(shard_id, _)| Self::task_assignable_to_shard(task_id, *shard_id))
					.map(|(shard_id, _)| (shard_id, Self::shard_task_count(shard_id)))
					.reduce(|(shard_id, task_count), (shard_id2, task_count2)| {
						if task_count < task_count2 {
							(shard_id, task_count)
						} else {
							(shard_id2, task_count2)
						}
					});
				let Some((shard_id, _)) = shard else {
					break;
				};

				if Self::is_payable(task_id)
					&& !matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Read(Some(_)))
					&& !matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Sign)
				{
					TaskPhaseState::<T>::insert(
						task_id,
						TaskPhase::Write(T::Shards::random_signer(shard_id)),
					);
					WritePhaseStart::<T>::insert(
						task_id,
						frame_system::Pallet::<T>::block_number(),
					);
				}
				ShardTasks::<T>::insert(shard_id, task_id, ());
				TaskShard::<T>::insert(task_id, shard_id);
				UnassignedTasks::<T>::remove(network, task_id);
			}
		}

		fn register_shard_scheduled(shard_id: ShardId) -> bool {
			RegisterShardScheduled::<T>::get(shard_id).is_some()
		}

		pub fn get_task_cycle(task_id: TaskId) -> TaskCycle {
			TaskCycleState::<T>::get(task_id)
		}

		pub fn get_task_phase(task_id: TaskId) -> TaskPhase {
			TaskPhaseState::<T>::get(task_id)
		}

		pub fn get_task_shard(task_id: TaskId) -> Option<ShardId> {
			TaskShard::<T>::get(task_id)
		}

		pub fn get_task_results(
			task_id: TaskId,
			cycle: Option<TaskCycle>,
		) -> Vec<(TaskCycle, TaskResult)> {
			if let Some(cycle) = cycle {
				let mut results = vec![];
				if let Some(result) = TaskResults::<T>::get(task_id, cycle) {
					results.push((cycle, result))
				}
				results
			} else {
				TaskResults::<T>::iter_prefix(task_id).collect::<Vec<_>>()
			}
		}
	}

	impl<T: Config> TasksInterface for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::insert(network, shard_id, ());
			if !T::Shards::is_shard_registered(shard_id)
				&& !Self::register_shard_scheduled(shard_id)
			{
				// TODO: schedule task to register the shard
				// schedule calling this or is it like that is handled by executer?
				// shardid: u64, bytes: Vec<u8>, TssSignature signature
				// function submit(uint64, uint[] memory, uint[] memory) public {
				// make a task, wait for task completion
				// not just TaskResult vs listening for the event
				// call handler to gmp pallet
				RegisterShardScheduled::<T>::insert(shard_id, ());
			}
			Self::schedule_tasks(network);
		}

		fn shard_offline(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::remove(network, shard_id);
			ShardTasks::<T>::drain_prefix(shard_id).for_each(|(task_id, _)| {
				TaskShard::<T>::remove(task_id);
				if Self::is_runnable(task_id) || Self::is_resumable(task_id) {
					UnassignedTasks::<T>::insert(network, task_id, ());
				}
			});
			Self::schedule_tasks(network);
		}
	}
}
