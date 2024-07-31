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

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frameless::Networks;
	use scale_info::prelude::string::String;
	use scale_info::prelude::vec::Vec;
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
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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
				Networks::new().insert(name.clone(), network.clone());
			}
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
			let network_id = Networks::new().insert(chain_name, chain_network);
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
			Networks::new().get(network_id)
		}
	}
}
