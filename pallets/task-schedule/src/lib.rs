#![cfg_attr(not(feature = "std"), no_std)]
pub mod weights;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::vec::Vec;
	use time_primitives::abstraction::TaskSchedule;
	pub type KeyId = u64;

	pub trait WeightInfo {
		fn store_schedule() -> Weight;
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
		StorageMap<_, Blake2_128Concat, KeyId, TaskSchedule, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		ScheduleStored(KeyId),

		///Already exist case
		AlreadyExist(KeyId),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::store_schedule())]
		pub fn insert_schedule(origin: OriginFor<T>, schedule: TaskSchedule) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let last_key = self::ScheduleStorage::<T>::iter_keys().last();
			let schedule_id = match last_key {
				Some(val) => val + 1,
				None => 1,
			};
			self::ScheduleStorage::<T>::insert(schedule_id, schedule.clone());
			Self::deposit_event(Event::ScheduleStored(schedule_id));

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn get_schedules() -> Result<Vec<TaskSchedule>, DispatchError> {
			let data_list = self::ScheduleStorage::<T>::iter_values().collect::<Vec<_>>();
			// will add scheduling logic

			Ok(data_list)
		}
		pub fn get_schedules_keys() -> Result<Vec<u64>, DispatchError> {
			let data_list = self::ScheduleStorage::<T>::iter_keys().collect::<Vec<_>>();
			// will add scheduling logic

			Ok(data_list)
		}
		pub fn get_schedule_by_key(key: u64) -> Result<Option<TaskSchedule>, DispatchError> {
			let data = self::ScheduleStorage::<T>::get(key);
			// will add scheduling logic

			Ok(data)
		}
	}
}
