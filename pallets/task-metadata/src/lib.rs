#![cfg_attr(not(feature = "std"), no_std)]
pub mod weights;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::{string::String, vec::Vec};
	use time_primitives::abstraction::{Collection, Task};
	pub type KeyId = u64;

	pub trait WeightInfo {
		fn store_task() -> Weight;
		fn store_collection() -> Weight;
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
	#[pallet::getter(fn get_task_metadata)]
	pub(super) type TaskMeta<T: Config> = StorageMap<_, Blake2_128Concat, KeyId, Task, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_collection_metadata)]
	pub(super) type CollectionMeta<T: Config> =
		StorageMap<_, Blake2_128Concat, String, Collection, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the record id that uniquely identify
		TaskMetaStored(KeyId),

		///Already exist case
		AlreadyExist(KeyId),

		/// Collections
		ColMetaStored(String),

		///Already exist case
		ColAlreadyExist(String),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::weight(T::WeightInfo::store_task())]
		pub fn insert_task(origin: OriginFor<T>, task: Task) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let data_list = self::TaskMeta::<T>::get(task.collection_id.0);
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

		#[pallet::weight(T::WeightInfo::store_collection())]
		pub fn insert_collection(
			origin: OriginFor<T>,
			hash: String,
			task: Vec<u8>,
			validity: i64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let data_list = self::CollectionMeta::<T>::get(&hash);
			match data_list {
				Some(val) => {
					Self::deposit_event(Event::ColAlreadyExist(val.hash));
				},
				None => {
					self::CollectionMeta::<T>::insert(
						hash.clone(),
						Collection {
							hash: hash.clone(),
							task,
							validity,
						},
					);
					Self::deposit_event(Event::ColMetaStored(hash));
				},
			}

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn get_task_by_key(key: KeyId) -> Result<Option<Task>, DispatchError> {
			let data_list = self::TaskMeta::<T>::get(key);
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
