#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub use pallet::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	pub trait WeightInfo {
		fn add_network() -> Weight;
	}

	impl WeightInfo for () {
		fn add_network() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = sp_runtime::AccountId32> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NetworkAdded,
	}

	#[pallet::error]
	pub enum Error<T> {
		NetworkAlreadyExist,
	}

	#[pallet::storage]
	/// Counter for creating unique network during on-chain creation
	pub type NetworkIdCounter<T: Config> = StorageValue<_, u64, ValueQuery>;
	/// Network for which shards can be assigned tasks
	#[pallet::storage]
	pub type NetworkBlockchain<T: Config> =
		StorageMap<_, Blake2_128Concat, u64, String, OptionQuery>;

	/// Network for which shards can be assigned tasks
	#[pallet::storage]
	pub type NetworkName<T: Config> = StorageMap<_, Blake2_128Concat, u64, String, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_network())]
		pub fn add_network(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;
			Self::deposit_event(Event::NetworkAdded);
			Ok(())
		}
	}
}
