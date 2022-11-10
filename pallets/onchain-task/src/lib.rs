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
		/// The chain id that uniquely identify the chain data
		OnchainTaskStored(SupportedChain, Vec<OnchainTaskData>),
		
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The chain is not known
		UnknownChain,
	}


	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing onchain data
		#[pallet::weight(
			T::WeightInfo::store_onchain_task()
		)]
		pub fn store_onchain_task(
			origin: OriginFor<T>,
			chain: SupportedChain,
			chain_id: ChainId,
			chain_data: ChainData,
			methods: TaskMethod,	
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let mut onchain_data = Vec::new();
			let onchain_task = OnchainTaskData {
				chain_id: chain_id.clone(),
				chain_data: chain_data.clone(),
				methods,
			};
			onchain_data.push(onchain_task.clone());
			<OnchainTaskStore<T>>::append(
				chain.clone(), onchain_task.clone(),
			);

			Self::deposit_event(Event::OnchainTaskStored(chain.clone(), onchain_data.clone()));

			Ok(())
		}


	}

}
