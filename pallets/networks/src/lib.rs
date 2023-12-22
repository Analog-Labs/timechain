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
	use time_primitives::{NetworkBlockchain, NetworkId, NetworkName};

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
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type MaxBlockchainSize: Get<u32>;
		type MaxNameSize: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NetworkAdded(NetworkId),
	}

	#[pallet::error]
	pub enum Error<T> {
		NetworkAlreadyExists,
		BlockchainTooLong,
		NameTooLong,
	}

	// stores a counter for each network type supported
	#[pallet::storage]
	pub type NetworkIdCounter<T: Config> = StorageValue<_, NetworkId, ValueQuery>;

	// stores blockchain against its supported types Vec<Networks>
	#[pallet::storage]
	pub(super) type NetworkNames<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BoundedVec<u8, T::MaxBlockchainSize>,
		Vec<BoundedVec<u8, T::MaxNameSize>>,
		ValueQuery,
	>;

	// stores network_id against (blockchain, network)
	#[pallet::storage]
	pub(super) type NetworkIdToNetwork<T: Config> = StorageMap<
		_,
		Twox64Concat,
		NetworkId,
		(BoundedVec<u8, T::MaxBlockchainSize>, BoundedVec<u8, T::MaxNameSize>),
		OptionQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_network())]
		pub fn add_network(
			origin: OriginFor<T>,
			blockchain: NetworkBlockchain,
			name: NetworkName,
		) -> DispatchResult {
			ensure_root(origin)?;

			let bounded_blockchain = BoundedVec::<u8, T::MaxBlockchainSize>::try_from(
				blockchain.trim().to_string().into_bytes(),
			)
			.map_err(|_| Error::<T>::BlockchainTooLong)?;

			let bounded_name =
				BoundedVec::<u8, T::MaxNameSize>::try_from(name.trim().to_string().into_bytes())
					.map_err(|_| Error::<T>::NameTooLong)?;

			let mut networks = NetworkNames::<T>::get(&bounded_blockchain);
			ensure!(!networks.contains(&bounded_name), <Error<T>>::NetworkAlreadyExists);

			let network_id = <NetworkIdCounter<T>>::get();
			<NetworkIdCounter<T>>::put(network_id.saturating_add(1));

			networks.push(bounded_name.clone());
			NetworkNames::<T>::insert(&bounded_blockchain, networks);

			NetworkIdToNetwork::<T>::insert(network_id, (bounded_blockchain, bounded_name));
			Self::deposit_event(Event::NetworkAdded(network_id));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_network(id: NetworkId) -> Option<(NetworkBlockchain, NetworkName)> {
			NetworkIdToNetwork::<T>::get(id).map(|(bounded_blockchain, bounded_name)| {
				let blockchain = bounded_blockchain.to_vec();
				let name = bounded_name.to_vec();
				let blockchain_str = String::from_utf8(blockchain).unwrap_or("".into());
				let network_str = String::from_utf8(name).unwrap_or("".into());
				(blockchain_str, network_str)
			})
		}
	}
}
