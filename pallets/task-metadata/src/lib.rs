#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::vec::Vec;
	use time_primitives::abstraction::Task;
	pub type KeyId = u64;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_task_metadata)]
	pub(super) type TaskMeta<T: Config> = StorageMap<_, Blake2_128Concat, KeyId, Task, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the record id that uniquely identify
		TaskMetaStored(KeyId),

		///Already exist case
		AlreadyExist(KeyId),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100_000)]
		pub fn insert_task(origin: OriginFor<T>, task: Task) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let data_list =
				self::TaskMeta::<T>::iter_values().find(|x| x.collection_id == task.collection_id);
			match data_list {
				Some(val) => {
					Self::deposit_event(Event::AlreadyExist(val.collection_id.0));
				},
				None => {
					self::TaskMeta::<T>::insert(task.collection_id.0, task.clone());
					Self::deposit_event(Event::TaskMetaStored(task.collection_id.0));
				},
			}

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn get_task_by_key(key: KeyId) -> Result<Option<Task>, DispatchError> {
			let data_list = self::TaskMeta::<T>::iter_values().find(|x| x.collection_id.0 == key);
			match data_list {
				Some(val) => Ok(Some(val)),
				None => Ok(None),
			}
		}
		pub fn get_tasks() -> Result<Vec<Task>, DispatchError> {
			let data_list = self::TaskMeta::<T>::iter_values().collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_tasks_keys() -> Result<Vec<u64>, DispatchError> {
			let data_list = self::TaskMeta::<T>::iter_keys().collect::<Vec<_>>();

			Ok(data_list)
		}
	}
}
