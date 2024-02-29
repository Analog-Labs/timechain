#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::string::String;
	use scale_info::prelude::vec::Vec;
	use time_primitives::{ChainName, ChainNetwork, NetworkEvents, NetworkId, NetworksInterface};

	pub trait WeightInfo {
		fn add_network(name: usize, network: usize) -> Weight;
	}

	impl WeightInfo for () {
		fn add_network(_name: usize, _network: usize) -> Weight {
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
		type NetworkEvents: NetworkEvents;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NetworkAdded(NetworkId),
	}

	#[pallet::error]
	pub enum Error<T> {
		NetworkIdOverflow,
		NetworkExists,
	}

	// stores a counter for each network type supported
	#[pallet::storage]
	pub type NetworkIdCounter<T: Config> = StorageValue<_, NetworkId, ValueQuery>;

	// stores network_id against (blockchain, network)
	#[pallet::storage]
	pub type Networks<T: Config> =
		StorageMap<_, Twox64Concat, NetworkId, (ChainName, ChainNetwork), OptionQuery>;

	#[pallet::storage]
	pub type BlockHeight<T: Config> = StorageMap<_, Twox64Concat, NetworkId, u64, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		pub networks: Vec<(String, String)>,
		pub _marker: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				networks: Default::default(),
				_marker: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (name, network) in &self.networks {
				Pallet::<T>::insert_network(name.clone(), network.clone()).unwrap();
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn insert_network(
			chain_name: ChainName,
			chain_network: ChainNetwork,
		) -> Result<NetworkId, Error<T>> {
			for (_, (name, network)) in Networks::<T>::iter() {
				if name == chain_name && network == chain_network {
					return Err(Error::<T>::NetworkExists);
				}
			}

			let network_id = NetworkIdCounter::<T>::get();
			let Some(next_network_id) = network_id.checked_add(1) else {
				return Err(Error::<T>::NetworkIdOverflow);
			};

			NetworkIdCounter::<T>::put(next_network_id);
			Networks::<T>::insert(network_id, (chain_name, chain_network));

			Ok(network_id)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_network(chain_name.len(), chain_network.len()))]
		pub fn add_network(
			origin: OriginFor<T>,
			chain_name: ChainName,
			chain_network: ChainNetwork,
		) -> DispatchResult {
			ensure_root(origin)?;
			let network_id = Self::insert_network(chain_name, chain_network)?;
			Self::deposit_event(Event::NetworkAdded(network_id));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_network(network_id: NetworkId) -> Option<(ChainName, ChainNetwork)> {
			Networks::<T>::get(network_id)
		}
	}

	impl<T: Config> NetworksInterface for Pallet<T> {
		fn seen_block_height(network_id: NetworkId, block_height: u64) {
			let current = BlockHeight::<T>::get(network_id);
			if block_height > current {
				BlockHeight::<T>::set(network_id, block_height);
				T::NetworkEvents::block_height_changed(network_id, block_height);
			}
		}
	}
}
