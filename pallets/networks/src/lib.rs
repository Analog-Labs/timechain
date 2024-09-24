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
		fn register_network(name: u32, network: u32) -> Weight;
		fn set_network_config() -> Weight;
	}

	impl WeightInfo for () {
		fn register_network(_name: u32, _network: u32) -> Weight {
			Weight::default()
		}

		fn set_network_config() -> Weight {
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
		/// Network registered.
		NetworkRegistered(NetworkId, Address, u64),
		/// Network config changed.
		NetworkConfigChanged(NetworkId, u32, u32, u128, u32),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Network exists.
		NetworkExists,
		/// Network doesn't exist.
		NetworkNotFound,
	}

	/// Workaround for subxt not supporting iterating over the decoded keys.
	#[pallet::storage]
	pub type Networks<T: Config> = StorageMap<_, Twox64Concat, NetworkId, NetworkId, OptionQuery>;

	#[pallet::storage]
	pub type NetworkName<T: Config> =
		StorageMap<_, Twox64Concat, NetworkId, (ChainName, ChainNetwork), OptionQuery>;

	/// Map storage for network gateways.
	#[pallet::storage]
	pub type NetworkGatewayAddress<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, Address, OptionQuery>;

	#[pallet::storage]
	pub type NetworkGatewayBlock<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u64, OptionQuery>;

	///  Map storage for network batch sizes.
	#[pallet::storage]
	pub type NetworkBatchSize<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u32, OptionQuery>;

	/// Map storage for network offsets.
	#[pallet::storage]
	pub type NetworkBatchOffset<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u32, ValueQuery>;

	/// Map storage for batch gas limit.
	#[pallet::storage]
	pub type NetworkBatchGasLimit<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u128, OptionQuery>;

	/// Map storage for shard task limits.
	#[pallet::storage]
	pub type NetworkShardTaskLimit<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u32, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		pub networks: Vec<(NetworkId, String, String, Address, u64)>,
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
			for (id, blockchain, network, gateway, block) in &self.networks {
				Pallet::<T>::insert_network(*id, blockchain.clone(), network.clone(), *gateway, *block)
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
			gateway: Address,
			block_height: u64,
		) -> Result<(), Error<T>> {
			if Networks::<T>::get(network).is_some() {
				return Err(Error::<T>::NetworkExists);
			}
			Networks::<T>::insert(network, network);
			NetworkName::<T>::insert(network, (chain_name, chain_network));
			NetworkGatewayAddress::<T>::insert(network, gateway);
			NetworkGatewayBlock::<T>::insert(network, block_height);
			T::Tasks::gateway_registered(network, block_height);
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
		#[pallet::weight(T::WeightInfo::register_network(chain_name.len() as u32, chain_network.len() as u32))]
		pub fn register_network(
			origin: OriginFor<T>,
			network: NetworkId,
			chain_name: ChainName,
			chain_network: ChainNetwork,
			gateway: Address,
			block_height: u64,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Self::insert_network(network, chain_name, chain_network, gateway, block_height)?;
			Self::deposit_event(Event::NetworkRegistered(network, gateway, block_height));
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
		#[pallet::weight(<T as Config>::WeightInfo::set_network_config())]
		pub fn set_network_config(
			origin: OriginFor<T>,
			network: NetworkId,
			batch_size: u32,
			batch_offset: u32,
			batch_gas_limit: u128,
			shard_task_limit: u32,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			NetworkBatchSize::<T>::insert(network, batch_size);
			NetworkBatchOffset::<T>::insert(network, batch_offset);
			NetworkBatchGasLimit::<T>::insert(network, batch_gas_limit);
			NetworkShardTaskLimit::<T>::insert(network, shard_task_limit);
			Self::deposit_event(Event::NetworkConfigChanged(
				network,
				batch_size,
				batch_offset,
				batch_gas_limit,
				shard_task_limit,
			));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		///  Retrieves the network information (i.e., `ChainName` and `ChainNetwork`) associated with a given `NetworkId`.
		///    
		///  # Flow
		///  1. Call [`Networks`] to fetch the network information.
		///  2. Return the network information if it exists, otherwise return `None`.
		pub fn get_network(network: NetworkId) -> Option<(ChainName, ChainNetwork)> {
			NetworkName::<T>::get(network)
		}
	}

	impl<T: Config> NetworksInterface for Pallet<T> {
		fn gateway(network: NetworkId) -> Option<Address> {
			NetworkGatewayAddress::<T>::get(network)
		}

		fn next_batch_size(network: NetworkId, block_height: u64) -> u32 {
			const DEFAULT_BATCH_SIZE: u32 = 32;
			let network_batch_size =
				NetworkBatchSize::<T>::get(network).unwrap_or(DEFAULT_BATCH_SIZE);
			let network_offset = NetworkBatchOffset::<T>::get(network);
			network_batch_size
				- ((block_height + network_offset as u64) % network_batch_size as u64) as u32
		}

		fn batch_gas_limit(network: NetworkId) -> u128 {
			const DEFAULT_BATCH_GAS_LIMIT: u128 = 10_000;
			NetworkBatchGasLimit::<T>::get(network).unwrap_or(DEFAULT_BATCH_GAS_LIMIT)
		}

		fn shard_task_limit(network: NetworkId) -> u32 {
			const DEFAULT_SHARD_TASK_LIMIT: u32 = 10;
			NetworkShardTaskLimit::<T>::get(network).unwrap_or(DEFAULT_SHARD_TASK_LIMIT)
		}
	}
}
