#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::vec::Vec;
	use time_primitives::abstraction::{ObjectId, ScheduleInput, ScheduleStatus, TaskSchedule};
	pub type KeyId = u64;
	pub type ScheduleResults<AccountId> = Vec<(KeyId, TaskSchedule<AccountId>)>;
	pub trait WeightInfo {
		fn insert_schedule() -> Weight;
		fn update_schedule() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_task_schedule)]
	pub(super) type ScheduleStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, KeyId, TaskSchedule<T::AccountId>, OptionQuery>;

	#[pallet::storage]
	pub(super) type LastKey<T: Config> = StorageValue<_, u64, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		ScheduleStored(KeyId),

		/// Updated Schedule
		ScheduleUpdated(KeyId),

		///Already exist case
		AlreadyExist(KeyId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The signing account has no permission to do the operation.
		NoPermission,
		/// Error getting schedule ref.
		ErrorRef,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::insert_schedule())]
		pub fn insert_schedule(origin: OriginFor<T>, schedule: ScheduleInput) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let last_key = self::LastKey::<T>::get();
			let schedule_id = match last_key {
				Some(val) => val + 1,
				None => 1,
			};
			self::LastKey::<T>::put(schedule_id);
			self::ScheduleStorage::<T>::insert(
				schedule_id,
				TaskSchedule {
					task_id: schedule.task_id,
					owner: who,
					shard_id: schedule.shard_id,
					cycle: schedule.cycle,
					validity: schedule.validity,
					hash: schedule.hash,
					status: ScheduleStatus::Initiated,
				},
			);
			Self::deposit_event(Event::ScheduleStored(schedule_id));

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::update_schedule())]
		pub fn update_schedule(
			origin: OriginFor<T>,
			status: ScheduleStatus,
			key: KeyId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let _ = self::ScheduleStorage::<T>::try_mutate(key, |schedule| -> DispatchResult {
				let details = schedule.as_mut().ok_or(Error::<T>::ErrorRef)?;
				ensure!(details.owner == who, Error::<T>::NoPermission);

				details.status = status;
				Ok(())
			});
			Self::deposit_event(Event::ScheduleUpdated(key));

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn get_schedules() -> Result<ScheduleResults<T::AccountId>, DispatchError> {
			let data_list = self::ScheduleStorage::<T>::iter()
				.filter(|item| item.1.status == ScheduleStatus::Initiated)
				.collect::<Vec<_>>();

			Ok(data_list)
		}
		pub fn get_schedules_by_task_id(
			key: ObjectId,
		) -> Result<Vec<TaskSchedule<T::AccountId>>, DispatchError> {
			let data = self::ScheduleStorage::<T>::iter_values()
				.filter(|item| item.task_id == key)
				.collect::<Vec<_>>();

			Ok(data)
		}
		pub fn get_schedules_keys() -> Result<Vec<u64>, DispatchError> {
			let data_list = self::ScheduleStorage::<T>::iter_keys().collect::<Vec<_>>();

			Ok(data_list)
		}
		pub fn get_schedule_by_key(
			key: u64,
		) -> Result<Option<TaskSchedule<T::AccountId>>, DispatchError> {
			let data = self::ScheduleStorage::<T>::get(key);

			Ok(data)
		}

		pub fn update_schedule_by_key(
			status: ScheduleStatus,
			key: KeyId,
		) -> Result<(), DispatchError> {
			let _ = self::ScheduleStorage::<T>::try_mutate(key, |schedule| -> DispatchResult {
				let details = schedule.as_mut().ok_or(Error::<T>::ErrorRef)?;
				details.status = status;
				Ok(())
			});

			Ok(())
		}
	}
}
