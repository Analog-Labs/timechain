#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
pub mod types;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {

	use crate::{types::*, weights::WeightInfo};
	use frame_support::{pallet_prelude::*, traits::IsType};
	use frame_system::pallet_prelude::*;
	use sp_std::prelude::*;
	use itertools::Itertools;

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
	#[pallet::getter(fn next_task_id)]
	pub(super) type NextTaskId<T: Config> = StorageValue<_, TaskId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_metadata)]
	pub(super) type TaskMetadata<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, OnChainTaskMetadata, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_metadata_id)]
	pub(super) type TaskMetadataId<T: Config> =
		StorageMap<_, Blake2_128Concat, OnChainTaskMetadata, TaskId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_store)]
	pub(super) type OnchainTaskStore<T: Config> =
		StorageMap<_, Blake2_128Concat, SupportedChain, Vec<OnchainTask>, OptionQuery>;

	impl<T: Config> Pallet<T> {
		pub fn get_task_store() {
			OnchainTaskStore::<T>::iter().collect_vec();
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Emitted when the onchain task is stored successfully
		OnchainTaskStored(T::AccountId, SupportedChain, TaskId, OnChainTaskMetadata, Frequency),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No new task id available
		TaskIdOverflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing onchain task
		#[pallet::weight(T::WeightInfo::store_task())]
		pub fn store_task(
			origin: OriginFor<T>,
			chain: SupportedChain,
			task_metadata: OnChainTaskMetadata,
			frequency: Frequency,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let task_id = Self::task_metadata_id(&task_metadata);

			let new_task_id = match task_id {
				// task already exists before
				Some(id) => {
					Self::insert_task(chain, id, frequency);
					id
				},
				// new task
				None => {
					let task_id = Self::get_next_task_id()?;

					<TaskMetadata<T>>::insert(task_id, task_metadata.clone());
					<TaskMetadataId<T>>::insert(task_metadata.clone(), task_id);
					Self::insert_task(chain, task_id, frequency);
					task_id
				},
			};

			Self::deposit_event(Event::OnchainTaskStored(
				caller,
				chain,
				new_task_id,
				task_metadata,
				frequency,
			));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_next_task_id() -> Result<TaskId, Error<T>> {
			match Self::next_task_id() {
				Some(TaskId::MAX) => Err(Error::<T>::TaskIdOverflow),
				Some(id) => {
					<NextTaskId<T>>::put(id + 1);
					Ok(id + 1)
				},
				None => {
					<NextTaskId<T>>::put(0);
					Ok(0)
				},
			}
		}

		pub fn insert_task(chain: SupportedChain, task_id: TaskId, frequency: Frequency) {
			// build the object
			let task = OnchainTask { task_id, frequency };

			match Self::task_store(chain) {
				Some(ref mut tasks) => {
					match tasks.binary_search(&task) {
						Ok(index) => {
							// update frequency if new one is smaller
							if tasks[index].frequency > frequency {
								tasks[index].frequency = frequency
							}
						},
						Err(_) => {
							// not found then insert the new one and sort the tasks
							tasks.push(task);
							tasks.sort();
							<OnchainTaskStore<T>>::insert(chain, tasks);
						},
					}
				},
				None => {
					<OnchainTaskStore<T>>::insert(chain, vec![task]);
				},
			};
		}
	}
}
