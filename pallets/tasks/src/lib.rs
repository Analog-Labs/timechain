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
	use sp_runtime::Saturating;
	use sp_std::vec::Vec;
	use time_primitives::{
		CycleStatus, Network, OcwSubmitTaskResult, ScheduleInterface, ShardId,
		ShardStatusInterface, TaskCycle, TaskDescriptor, TaskDescriptorParams, TaskError,
		TaskExecution, TaskId, TaskStatus,
	};

	pub trait WeightInfo {
		fn create_task() -> Weight;
		fn stop_task() -> Weight;
		fn resume_task() -> Weight;
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
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = sp_runtime::AccountId32> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type ShardStatus: ShardStatusInterface;
		#[pallet::constant]
		type MaxRetryCount: Get<u8>;
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
	pub type Tasks<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskDescriptor, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_state)]
	pub type TaskState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskStatus, OptionQuery>;

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
		CycleStatus,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Updated cycle status
		TaskResult(TaskId, TaskCycle, CycleStatus),
		/// Error occured while executing task
		TaskError(TaskId, TaskError),
		/// Task stopped by owner
		TaskStopped(TaskId),
		/// Task resumed by owner
		TaskResumed(TaskId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Task
		UnknownTask,
		/// Invalid Task State
		InvalidTaskState,
		/// Invalid Owner
		InvalidOwner,
		/// Invalid cycle
		InvalidCycle,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_task())]
		pub fn create_task(origin: OriginFor<T>, schedule: TaskDescriptorParams) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let task_id = TaskIdCounter::<T>::get();
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
			let owner = ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(task.owner == owner, Error::<T>::InvalidOwner);
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
		pub fn resume_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(task.owner == owner, Error::<T>::InvalidOwner);
			ensure!(
				matches!(
					TaskState::<T>::get(task_id),
					Some(TaskStatus::Stopped) | Some(TaskStatus::Failed { .. })
				),
				Error::<T>::InvalidTaskState
			);
			TaskState::<T>::insert(task_id, TaskStatus::Created);
			Self::deposit_event(Event::TaskResumed(task_id));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution> {
			ShardTasks::<T>::iter_prefix(shard_id)
				.filter(|(task_id, _)| Self::is_runnable(*task_id))
				.map(|(task_id, _)| {
					TaskExecution::new(
						task_id,
						TaskCycleState::<T>::get(task_id),
						TaskRetryCounter::<T>::get(task_id),
					)
				})
				.collect()
		}

		pub fn get_task(task_id: TaskId) -> Option<TaskDescriptor> {
			Tasks::<T>::get(task_id)
		}
	}

	impl<T: Config> Pallet<T> {
		fn is_complete(task_id: TaskId) -> bool {
			if let Some(task) = Tasks::<T>::get(task_id) {
				TaskResults::<T>::contains_key(task_id, task.cycle)
			} else {
				true
			}
		}

		fn is_resumable(task_id: TaskId) -> bool {
			matches!(TaskState::<T>::get(task_id), Some(TaskStatus::Stopped) | Some(TaskStatus::Failed { .. }))
		}

		fn is_runnable(task_id: TaskId) -> bool {
			matches!(TaskState::<T>::get(task_id), Some(TaskStatus::Created))
		}

		fn shard_task_count(shard_id: ShardId) -> usize {
			ShardTasks::<T>::iter_prefix(shard_id).count()
		}

		fn schedule_tasks(network: Network) {
			for (task_id, _) in UnassignedTasks::<T>::iter_prefix(network) {
				let shard = NetworkShards::<T>::iter_prefix(network)
					.filter(|(shard_id, _)| T::ShardStatus::is_shard_online(*shard_id))
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
				ShardTasks::<T>::insert(shard_id, task_id, ());
				TaskShard::<T>::insert(task_id, shard_id);
				UnassignedTasks::<T>::remove(network, task_id);
			}
		}
	}

	impl<T: Config> ScheduleInterface for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::insert(network, shard_id, ());
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

	impl<T: Config> OcwSubmitTaskResult for Pallet<T> {
		fn submit_task_result(
			task_id: TaskId,
			cycle: TaskCycle,
			status: CycleStatus,
		) -> DispatchResult {
			ensure!(TaskCycleState::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			let incremented_cycle = cycle.saturating_plus_one();
			TaskCycleState::<T>::insert(task_id, incremented_cycle);
			TaskResults::<T>::insert(task_id, incremented_cycle, status.clone());
			TaskRetryCounter::<T>::insert(task_id, 0);
			if Self::is_complete(task_id) {
				if let Some(shard_id) = TaskShard::<T>::take(task_id) {
					ShardTasks::<T>::remove(shard_id, task_id);
				}
				TaskState::<T>::insert(task_id, TaskStatus::Completed);
			}
			Self::deposit_event(Event::TaskResult(task_id, incremented_cycle, status));
			Ok(())
		}

		fn submit_task_error(task_id: TaskId, error: TaskError) -> DispatchResult {
			let retry_count = TaskRetryCounter::<T>::get(task_id);
			TaskRetryCounter::<T>::insert(task_id, retry_count.saturating_plus_one());
			//since count starts from 0 decrementing maxretrycount
			if retry_count == T::MaxRetryCount::get() - 1 {
				TaskState::<T>::insert(task_id, TaskStatus::Failed { error: error.clone() });
				Self::deposit_event(Event::TaskError(task_id, error));
			}
			Ok(())
		}
	}
}
