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
	#[pallet::getter(fn task_store)]
	pub(super) type OnchainTaskStore<T: Config> =
		StorageMap<_, Blake2_128Concat, SupportedChain, Vec<OnchainTaskData>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Emitted when the onchain task is stored successfully
		OnchainTaskStored(SupportedChain, OnchainTaskData),

		/// Emitted when the onchain tasks 
		/// for a supported chain is removed successfully
		OnchainTaskRemoved(SupportedChain),
		
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The task is not known
		UnknownTask,
		/// Task not found
		TaskNotFound,
		/// Empty task error
		EmptyTask,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing onchain task
		#[pallet::weight(
			T::WeightInfo::store_onchain_task()
		)]
		pub fn store_onchain_task(
			origin: OriginFor<T>,
			chain: SupportedChain,
			chain_data: OnchainTaskData,	
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			ensure!(chain_data.task.len() > 0, Error::<T>::EmptyTask);
			<OnchainTaskStore<T>>::append(
				chain.clone(), chain_data.clone(),
			);

			Self::deposit_event(Event::OnchainTaskStored(chain.clone(), chain_data.clone()));

			Ok(())
		}

		/// Extrinsic for removing onchain all tasks
		/// for a supported chain
		/// Callable only by root for now
		#[pallet::weight(T::WeightInfo::remove_onchain_task())]
		pub fn remove_onchain_task(origin: OriginFor<T>, chain: SupportedChain) -> DispatchResult {
			let _ = ensure_root(origin)?;

			<OnchainTaskStore<T>>::remove(chain.clone());

			Self::deposit_event(Event::OnchainTaskRemoved(chain.clone()));

			Ok(())
		}

		#[pallet::weight(000)]
		pub fn edit_task_data(
			origin: OriginFor<T>,
			chain: SupportedChain,
			old_data: OnchainTaskData,
			new_data: OnchainTaskData,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let index = Self::get_task_index(chain.clone(), old_data.clone())?;
			let mut onchain_task = OnchainTaskStore::<T>::get(chain.clone()).ok_or(Error::<T>::UnknownTask)?;
			ensure!(old_data.clone() == onchain_task[index], Error::<T>::TaskNotFound);
			onchain_task[index] = new_data;
			OnchainTaskStore::<T>::insert(chain.clone(), onchain_task);
			Ok(())
		}

	}

	impl<T: Config> Pallet<T> {
		/// Helper functions to get the onchain task for a supported chain
		/// Also to get index of the task in the vector
		fn get_task_index(
			chain: SupportedChain,
			task_data: OnchainTaskData,
		) -> Result<usize, DispatchError> {
			let onchain_task = OnchainTaskStore::<T>::get(chain.clone()).ok_or(Error::<T>::UnknownTask)?;
			let index = onchain_task.iter().position(|r| r.clone() == task_data).unwrap();
			Ok(index)
		}

	}


}
