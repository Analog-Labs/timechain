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
	use frame_support::traits::{Randomness, IsType};
	use frame_support::sp_runtime::traits::Hash;
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
		type TaskRandomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_nonce)]
	pub(super) type Nonce<T: Config> = StorageValue<
	_,
	u64,
	ValueQuery
	>;

	#[pallet::storage]
	#[pallet::getter(fn tesseract_tasks)]
	pub type TesseractTasks<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, TesseractTask, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_store)]
	pub(super) type OnchainTaskStore<T: Config> =
		StorageMap<_, Blake2_128Concat, T::Hash, OnchainTaskData<T::Hash>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The chain id that uniquely identify the chain data
		OnchainDataStored(T::AccountId, T::Hash),

		/// A tesseract task has been added
		TesseractTaskAdded(T::AccountId, TesseractTask),

		/// A tesseract task removed
		TesseractTaskRemoved(T::AccountId),
		
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The Tesseract task is not known
		UnknownTask,

		/// Nonce has overflowed past u64 limits
		NonceOverflow,
	}

	impl<T: Config> OnchainTaskData<T>{}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing onchain data
		#[pallet::weight(
			T::WeightInfo::store_onchain_task()
		)]
		pub fn store_onchain_task(
			origin: OriginFor<T>,
			chain_data: ChainData,
			methods: Methods,	
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let random_hash = Self::random_hash(&caller.clone());
			ensure!(TesseractTasks::<T>::contains_key(caller.clone()), Error::<T>::UnknownTask);
			let onchain_task = OnchainTaskData {
				id: random_hash,
				chain_data,
				methods,
			};

			<OnchainTaskStore<T>>::insert(
				random_hash, onchain_task.clone(),
			);

			Self::deposit_event(Event::OnchainDataStored(caller.clone(), random_hash));

			Ok(())
		}

		/// Extrinsic for adding a node's task
		#[pallet::weight(T::WeightInfo::add_task())]
		pub fn add_task(
			origin: OriginFor<T>,
			account: T::AccountId,
			task: TesseractTask,
		) -> DispatchResult {
			_ = ensure_signed(origin)?;
			<TesseractTasks<T>>::insert(account.clone(), task.clone());

			Self::deposit_event(Event::TesseractTaskAdded(account, task));

			Ok(())
		}

		/// Extrinsic for adding a node's task
		#[pallet::weight(T::WeightInfo::remove_task())]
		pub fn remove_task(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			_ = ensure_signed(origin)?;

			<TesseractTasks<T>>::remove(account.clone());

			Self::deposit_event(Event::TesseractTaskRemoved(account));

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>{
		/// Helper function to add a tesseract task
        fn increment_nonce() -> DispatchResult{
			<Nonce<T>>::try_mutate(|nonce|{
				let next = nonce.checked_add(1).ok_or(Error::<T>::NonceOverflow)?;
				*nonce = next;
				Ok(().into())
			})
		}

        pub fn random_hash(sender: &T::AccountId) -> T::Hash{
			let nonce = <Nonce<T>>::get();
			let seed = T::TaskRandomness::random_seed();
			
			T::Hashing::hash_of(&(seed, sender, nonce))
		}

	}

}
