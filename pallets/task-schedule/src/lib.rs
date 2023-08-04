#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use pallet::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use time_primitives::{
		Network, ScheduleCycle, ScheduleInput, ScheduleStatus, ShardId, TaskId, TaskSchedule,
	};

	pub trait WeightInfo {
		fn create_task() -> Weight;
		fn submit_result() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = sp_runtime::AccountId32> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn unassigned_tasks)]
	pub type UnassignedTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_tasks)]
	pub type ShardTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_id_counter)]
	pub type TaskIdCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_definition)]
	pub type Tasks<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, TaskSchedule, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_cycle)]
	pub type TaskCycle<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, ScheduleCycle, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_shard)]
	pub type TaskShard<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, ShardId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_result)]
	pub type TaskResults<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TaskId,
		Blake2_128Concat,
		ScheduleCycle,
		ScheduleStatus,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Updated Schedule
		TaskResult(TaskId, ScheduleCycle, ScheduleStatus),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Invalid cycle
		InvalidCycle,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_task())]
		pub fn create_task(origin: OriginFor<T>, schedule: ScheduleInput) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let task_id = TaskIdCounter::<T>::get();
			Tasks::<T>::insert(
				task_id,
				TaskSchedule {
					owner: who,
					network: schedule.network,
					function: schedule.function,
					cycle: schedule.cycle,
					start: schedule.start,
					period: schedule.period,
					hash: schedule.hash,
				},
			);
			TaskIdCounter::<T>::put(task_id + 1);
			UnassignedTasks::<T>::insert(schedule.network, task_id, ());
			Self::deposit_event(Event::TaskCreated(task_id));
			Self::schedule_tasks();
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_result())]
		pub fn submit_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: ScheduleCycle,
			status: ScheduleStatus,
			// proof: Signature, TODO: add proof to authenticate
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(TaskCycle::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			TaskCycle::<T>::insert(task_id, cycle + 1);
			TaskResults::<T>::insert(task_id, cycle, status.clone());
			Self::deposit_event(Event::TaskResult(task_id, cycle, status));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_shard_tasks(shard_id: ShardId) -> Vec<(TaskId, ScheduleCycle)> {
			ShardTasks::<T>::iter_prefix(shard_id)
				.map(|(task_id, _)| (task_id, TaskCycle::<T>::get(task_id)))
				.collect()
		}

		pub fn get_task(task_id: TaskId) -> Option<TaskSchedule> {
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

		fn schedule_tasks() {
			todo!()
		}
	}
}
