#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::{string::String, vec::Vec};
	use time_primitives::{
		abstraction::{Collection, PayableTask, Task},
		ProxyExtend,
	};
	pub type KeyId = u64;

	pub trait WeightInfo {
		fn insert_task() -> Weight;
		fn insert_collection() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type ProxyExtend: ProxyExtend<Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_task_metadata)]
	pub type TaskMetaStorage<T: Config> = StorageMap<_, Blake2_128Concat, KeyId, Task, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_payable_task_metadata)]
	pub type PayableTaskMetaStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, KeyId, PayableTask, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_collection_metadata)]
	pub type CollectionMeta<T: Config> =
		StorageMap<_, Blake2_128Concat, String, Collection, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskMetaStored(KeyId),

		///Already exist case
		AlreadyExist(KeyId),

		/// Collections
		ColMetaStored(String),

		///Already exist case
		ColAlreadyExist(String),

		// Payable Task Meta
		PayableTaskMetaStorage(KeyId),

		PayableTaskMetaAlreadyExist(KeyId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not a valid submitter
		NotProxyAccount,
		/// Error getting schedule ref.
		ErrorRef,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::insert_task())]
		pub fn insert_task(origin: OriginFor<T>, task: Task) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let resp = T::ProxyExtend::proxy_exist(who);
			ensure!(resp, Error::<T>::NotProxyAccount);
			let data_list = TaskMetaStorage::<T>::get(task.task_id.0);
			match data_list {
				Some(val) => {
					Self::deposit_event(Event::AlreadyExist(val.task_id.0));
				},
				None => {
					TaskMetaStorage::<T>::insert(task.task_id.0, task.clone());
					Self::deposit_event(Event::TaskMetaStored(task.task_id.0));
				},
			}

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::insert_collection())]
		pub fn insert_collection(
			origin: OriginFor<T>,
			hash: String,
			task: Vec<u8>,
			validity: i64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let data_list = CollectionMeta::<T>::get(&hash);
			match data_list {
				Some(val) => {
					Self::deposit_event(Event::ColAlreadyExist(val.hash));
				},
				None => {
					CollectionMeta::<T>::insert(
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

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::insert_task())]
		pub fn insert_payable_task(origin: OriginFor<T>, task: PayableTask) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let resp = T::ProxyExtend::proxy_exist(who);
			ensure!(resp, Error::<T>::NotProxyAccount);
			let data_list = PayableTaskMetaStorage::<T>::get(task.task_id.0);
			match data_list {
				Some(val) => {
					Self::deposit_event(Event::PayableTaskMetaAlreadyExist(val.task_id.0));
				},
				None => {
					PayableTaskMetaStorage::<T>::insert(task.task_id.0, task.clone());
					Self::deposit_event(Event::PayableTaskMetaStorage(task.task_id.0));
				},
			}

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn get_task_by_key(key: KeyId) -> Result<Option<Task>, DispatchError> {
			let data_list = TaskMetaStorage::<T>::get(key);
			Ok(data_list)
		}

		pub fn get_tasks() -> Result<Vec<Task>, DispatchError> {
			let data_list = TaskMetaStorage::<T>::iter_values().collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_tasks_keys() -> Result<Vec<u64>, DispatchError> {
			let data_list = TaskMetaStorage::<T>::iter_keys().collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_payable_tasks() -> Result<Vec<PayableTask>, DispatchError> {
			let data_list = PayableTaskMetaStorage::<T>::iter_values().collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_payable_task_metadata_by_key(
			key: KeyId,
		) -> Result<Option<PayableTask>, DispatchError> {
			let data_list = PayableTaskMetaStorage::<T>::get(key);

			match data_list {
				Some(val) => Ok(Some(val)),
				None => Ok(None),
			}
		}
	}
}
