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
	use time_primitives::{
		Address, ChainName, ChainNetwork, NetworkId, NetworksInterface, TasksInterface,
	};

	pub trait WeightInfo {
		fn add_network(name: u32, network: u32) -> Weight;
		fn register_gateway() -> Weight;
		fn set_batch_size() -> Weight;
		fn set_batch_gas_limit() -> Weight;
	}

	impl WeightInfo for () {
		fn add_network(_name: u32, _network: u32) -> Weight {
			Weight::default()
		}

		fn register_gateway() -> Weight {
			Weight::default()
		}

		fn set_batch_size() -> Weight {
			Weight::default()
		}

		fn set_batch_gas_limit() -> Weight {
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
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type WeightInfo: WeightInfo;
		type Tasks: TasksInterface;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a new network is successfully added, providing the corresponding `NetworkId` as part of the event payload.
		NetworkAdded(NetworkId),
		/// Gateway was registered.
		GatewayRegistered(NetworkId, Address, u64),
		/// Batch size changed.
		BatchSizeChanged(NetworkId, u64, u64),
		/// Batch gas limit changed.
		BatchGasLimitChanged(NetworkId, u128),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error indicating that the generation of a new `NetworkId` has exceeded its maximum limit.
		NetworkIdOverflow,
		/// Error indicating that a network with the same `ChainName` and `ChainNetwork` already exists and cannot be added again.
		NetworkExists,
		/// Gateway was already registered.
		GatewayAlreadyRegistered,
	}

	// stores network_id against (blockchain, network)
	#[pallet::storage]
	pub type Networks<T: Config> =
		StorageMap<_, Twox64Concat, NetworkId, (ChainName, ChainNetwork), OptionQuery>;

	/// Map storage for network gateways.
	#[pallet::storage]
	pub type Gateway<T: Config> = StorageMap<_, Blake2_128Concat, NetworkId, Address, OptionQuery>;

	///  Map storage for network batch sizes.
	#[pallet::storage]
	pub type NetworkBatchSize<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u64, OptionQuery>;

	/// Map storage for network offsets.
	#[pallet::storage]
	pub type NetworkOffset<T: Config> = StorageMap<_, Blake2_128Concat, NetworkId, u64, ValueQuery>;

	/// Map storage for batch gas limit.
	#[pallet::storage]
	pub type NetworkBatchGasLimit<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u128, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		pub networks: Vec<(NetworkId, String, String)>,
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
			for (id, blockchain, network) in &self.networks {
				Pallet::<T>::insert_network(*id, blockchain.clone(), network.clone())
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
			network: NetworkId,
			chain_name: ChainName,
			chain_network: ChainNetwork,
		) -> Result<(), Error<T>> {
			if Networks::<T>::get(network).is_some() {
				return Err(Error::<T>::NetworkExists);
			}
			Networks::<T>::insert(network, (chain_name, chain_network));
			Ok(())
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
			network_id: NetworkId,
			chain_name: ChainName,
			chain_network: ChainNetwork,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Self::insert_network(network_id, chain_name, chain_network)?;
			Self::deposit_event(Event::NetworkAdded(network_id));
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::register_gateway())]
		pub fn register_gateway(
			origin: OriginFor<T>,
			network: NetworkId,
			address: Address,
			block_height: u64,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			ensure!(Gateway::<T>::get(network).is_none(), Error::<T>::GatewayAlreadyRegistered);
			Gateway::<T>::insert(network, address);
			Self::deposit_event(Event::GatewayRegistered(network, address, block_height));
			T::Tasks::gateway_registered(network, block_height);
			Ok(())
		}

		/// Sets the batch size and offset for a specific network.
		///
		/// # Flow
		///   1. Ensure the origin of the transaction is a root user.
		///   2. Insert the new batch size for the specified network into the [`NetworkBatchSize`] storage.
		///   3. Insert the new offset for the specified network into the [`NetworkOffset`] storage.
		///   4. Emit an event indicating the batch size and offset have been set.
		///   5. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::set_batch_size())]
		pub fn set_batch_size(
			origin: OriginFor<T>,
			network: NetworkId,
			batch_size: u64,
			offset: u64,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			NetworkBatchSize::<T>::insert(network, batch_size);
			NetworkOffset::<T>::insert(network, offset);
			Self::deposit_event(Event::BatchSizeChanged(network, batch_size, offset));
			Ok(())
		}

		/// Sets the batch gas limit
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::set_batch_size())]
		pub fn set_batch_gas_limit(
			origin: OriginFor<T>,
			network: NetworkId,
			batch_gas_limit: u128,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			NetworkBatchGasLimit::<T>::insert(network, batch_gas_limit);
			Self::deposit_event(Event::BatchGasLimitChanged(network, batch_gas_limit));
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

	impl<T: Config> NetworksInterface for Pallet<T> {
		fn gateway(network: NetworkId) -> Option<Address> {
			Gateway::<T>::get(network)
		}

		fn next_batch_size(network: NetworkId, block_height: u64) -> u64 {
			const DEFAULT_BATCH_SIZE: u64 = 32;
			let network_batch_size =
				NetworkBatchSize::<T>::get(network).unwrap_or(DEFAULT_BATCH_SIZE);
			let network_offset = NetworkOffset::<T>::get(network);
			network_batch_size - ((block_height + network_offset) % network_batch_size)
		}

		fn batch_gas_limit(network: NetworkId) -> u128 {
			const DEFAULT_BATCH_GAS_LIMIT: u128 = 10_000;
			NetworkBatchGasLimit::<T>::get(network).unwrap_or(DEFAULT_BATCH_GAS_LIMIT)
		}
	}
}
