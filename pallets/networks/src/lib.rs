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
		fn add_network(name: u32, network: u32) -> Weight;
	}

	impl WeightInfo for () {
		fn add_network(_name: u32, _network: u32) -> Weight {
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

	#[pallet::storage]
	pub type BlockSpeed<T: Config> = StorageMap<_, Twox64Concat, NetworkId, u64, ValueQuery>;

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
				Pallet::<T>::insert_network(name.clone(), network.clone())
					.expect("No networks exist before genesis; NetworkId not overflow from 0 at genesis; QED");
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let mut writes = 0;
			for (network, speed) in BlockSpeed::<T>::iter() {
				let height = BlockHeight::<T>::get(network);
				let next_height = height + speed;
				BlockHeight::<T>::insert(network, next_height);
				// NOTE: is not distributive since the rounding
				// is important.
				if next_height / 1000 - height / 1000 > 0 {
					T::NetworkEvents::block_height_changed(network, next_height / 1000);
				}
				writes += 1;
			}
			T::DbWeight::get().writes(writes)
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
		#[pallet::weight(T::WeightInfo::add_network(chain_name.len() as u32, chain_network.len() as u32))]
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
		fn seen_block_speed(network_id: NetworkId, block_speed: u64) {
			BlockSpeed::<T>::insert(network_id, block_speed);
		}
	}
}
