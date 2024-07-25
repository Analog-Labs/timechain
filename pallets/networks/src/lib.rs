#![cfg_attr(not(feature = "std"), no_std)]
//! The network pallet manages the registration and configuration of various
//! blockchain networks within a decentralized system. It provides functionality
//! to add new networks, retrieve network information, and ensures each network
//! is uniquely identified by a `NetworkId`.
//!
//! ## Features
//!
//! - Allows privileged users to add new blockchain networks with unique
//!   `ChainName` and `ChainNetwork` combinations. This operation requires root
//!   authorization.
//!
//! - Maintains storage of network configurations using a mapping between
//!   `NetworkId` and `(ChainName, ChainNetwork)` tuples.
//!
//! - Initializes the pallet with predefined network configurations during
//!   blockchain genesis, ensuring seamless operation from the start.
//!
//!
#![doc = simple_mermaid::mermaid!("../docs/network_flow.mmd")]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use polkadot_sdk::{frame_support, frame_system};

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::{string::String, vec::Vec};
	use time_primitives::{ChainName, ChainNetwork, NetworkId};

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
	pub trait Config: polkadot_sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a new network is successfully added, providing the corresponding `NetworkId` as part of the event payload.
		NetworkAdded(NetworkId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error indicating that the generation of a new `NetworkId` has exceeded its maximum limit.
		NetworkIdOverflow,
		/// Error indicating that a network with the same `ChainName` and `ChainNetwork` already exists and cannot be added again.
		NetworkExists,
	}

	// stores a counter for each network type supported
	#[pallet::storage]
	pub type NetworkIdCounter<T: Config> = StorageValue<_, NetworkId, ValueQuery>;

	// stores network_id against (blockchain, network)
	#[pallet::storage]
	pub type Networks<T: Config> =
		StorageMap<_, Twox64Concat, NetworkId, (ChainName, ChainNetwork), OptionQuery>;

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

	/// The pallet's genesis configuration (`GenesisConfig`) allows initializing the pallet with predefined network configurations. It specifies an initial list of `(ChainName, ChainNetwork)` pairs that are added to the storage during genesis.
	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (name, network) in &self.networks {
				Pallet::<T>::insert_network(name.clone(), network.clone())
					.expect("No networks exist before genesis; NetworkId not overflow from 0 at genesis; QED");
			}
		}
	}

	impl<T: Config> Pallet<T> {
		///  Inserts a new network into storage if it doesn't already exist. Updates the `NetworkIdCounter` to ensure unique network IDs.
		///    
		///  # Flow
		///    1. Iterate through existing networks to check if the given `ChainName` and `ChainNetwork` already exist.
		///    2. If the network exists, return [`Error::<T>::NetworkExists`].
		///    3. Retrieve the current [`NetworkIdCounter`].
		///    4. Increment the counter and check for overflow, returning [`Error::<T>::NetworkIdOverflow`] if overflow occurs.
		///    5. Update the [`NetworkIdCounter`] with the new value.
		///    6. Insert the new network into the [`Networks`] storage map with the current `NetworkId`.
		///    7. Return the new `NetworkId`.
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
		///  Adds a new blockchain network with a unique `ChainName` and `ChainNetwork` combination.
		///    
		///  # Flow
		///    
		///    1. Ensure the caller is the root user.
		///    2. Call `Self::insert_network(chain_name, chain_network).
		///    3. Emit the [`Event::NetworkAdded`] event with the new `NetworkId`.
		///    4. Return `Ok(())` to indicate success.
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
		///  Retrieves the network information (i.e., `ChainName` and `ChainNetwork`) associated with a given `NetworkId`.
		///    
		///  # Flow
		///  1. Call [`Networks`] to fetch the network information.
		///  2. Return the network information if it exists, otherwise return `None`.
		pub fn get_network(network_id: NetworkId) -> Option<(ChainName, ChainNetwork)> {
			Networks::<T>::get(network_id)
		}
	}
}
