#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
mod types;

pub mod weights;


#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_std::prelude::*;
	use frame_support::traits::IsType;
	use crate::weights::WeightInfo;
	use crate::types::*;
	
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
		StorageMap<_, Blake2_128Concat, SupportedChain, Vec<OnchainTasks>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Emitted when the onchain task is stored successfully
		OnchainTaskStored(SupportedChain, OnchainTasks),

		/// Emitted when onchain task is edited successfully
		OnchainTaskEdited(SupportedChain, OnchainTasks),

		/// Emitted when all onchain tasks 
		/// for a supported chain are removed successfully
		OnchainTasksRemoved(SupportedChain),

		/// Emitted when the onchain task is removed successfully
		OnchainTaskRemoved(SupportedChain, OnchainTasks),
		
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The task is not known
		UnknownTask,
		/// Task not found
		TaskNotFound,
		/// Empty task error
		EmptyTask,
		/// Task already exists
		TaskAlreadyExists,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing onchain task
		#[pallet::weight(
			T::WeightInfo::store_task()
		)]
		pub fn store_task(
			origin: OriginFor<T>,
			chain: SupportedChain,
			task_metadata: OnChainTaskMetadata,	
			frequency: Frequency,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let task_id = Self::task_metadata_id(&task_metadata);

			match task_id {
				// task already exists before
				Some(id) => {
					// just update the frequency with minimum value
					// <TaskMetadata<T>>::insert(id, &task_metadata);

				},
				// new task
				None => {
					// init id is zero
					let task_id = 0;
					<NextTaskId<T>>::put(task_id);
					<TaskMetadata<T>>::insert(task_id, &task_metadata);
					let task = OnchainTasks {
						task_id,
						frequency,
					};
					<OnchainTaskStore<T>>::append(&chain, task);

					let mut tasks = Self::task_store(&chain).unwrap();
					tasks.sort();

					<OnchainTaskStore<T>>::insert(&chain, tasks);
				}
			}
			
			Ok(())
		}	

	}

	impl<T: Config> Pallet<T> {

	}

}
