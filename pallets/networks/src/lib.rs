#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]
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
	use scale_info::prelude::vec::Vec;
	use time_primitives::{
		Address, ChainName, ChainNetwork, Network, NetworkConfig, NetworkId, NetworksInterface,
		TasksInterface,
	};

	pub trait WeightInfo {
		fn register_network(name: u32, network: u32) -> Weight;
		fn set_network_config() -> Weight;
		fn remove_network() -> Weight;
	}

	impl WeightInfo for () {
		fn register_network(_name: u32, _network: u32) -> Weight {
			Weight::default()
		}

		fn set_network_config() -> Weight {
			Weight::default()
		}

		fn remove_network() -> Weight {
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
		NetworkConfigChanged(NetworkId, NetworkConfig),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Network exists.
		NetworkExists,
		/// Network doesn't exist.
		NetworkNotFound,
		/// Network shard size is too large.
		ShardSizeAboveMax,
		/// Network threshold is invalid.
		ThresholdLargerThanSize,
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
		StorageMap<_, Blake2_128Concat, NetworkId, u32, OptionQuery>;

	/// Map storage for batch gas limit.
	#[pallet::storage]
	pub type NetworkBatchGasLimit<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u128, OptionQuery>;

	/// Map storage for shard task limits.
	#[pallet::storage]
	pub type NetworkShardTaskLimit<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u32, OptionQuery>;

	/// Map storage for shard size.
	#[pallet::storage]
	pub type NetworkShardSize<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u16, OptionQuery>;

	/// Map storage for shard size.
	#[pallet::storage]
	pub type NetworkShardThreshold<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u16, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		pub networks: Vec<Network>,
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
			for network in &self.networks {
				Pallet::<T>::insert_network(network)
					.expect("No networks exist before genesis; NetworkId not overflow from 0 at genesis; QED");
			}
		}
	}

	impl<T: Config> Pallet<T> {
		///  Inserts a new network into storage if it doesn't already exist.
		///    
		///  # Flow
		///    1. Iterate through existing networks to check if the given `ChainName` and `ChainNetwork` already exist.
		///    2. If the network exists, return [`Error::<T>::NetworkExists`].
		///    3. Insert the new network into the [`Networks`] storage map with the current `NetworkId`.
		///    4. Return the new `NetworkId`.
		fn insert_network(network: &Network) -> Result<(), Error<T>> {
			ensure!(network.id < NetworkId::MAX && Networks::<T>::get(network.id).is_none(), Error::<T>::NetworkExists);
			Networks::<T>::insert(network.id, network.id);
			NetworkName::<T>::insert(
				network.id,
				(network.chain_name.clone(), network.chain_network.clone()),
			);
			NetworkGatewayAddress::<T>::insert(network.id, network.gateway);
			NetworkGatewayBlock::<T>::insert(network.id, network.gateway_block);
			T::Tasks::gateway_registered(network.id, network.gateway_block);
			Self::insert_network_config(network.id, network.config.clone())?;
			Self::deposit_event(Event::NetworkRegistered(
				network.id,
				network.gateway,
				network.gateway_block,
			));
			Ok(())
		}

		fn insert_network_config(
			network: NetworkId,
			config: NetworkConfig,
		) -> Result<(), Error<T>> {
			ensure!(Networks::<T>::get(network).is_some(), Error::<T>::NetworkNotFound);
			ensure!(
				time_primitives::MAX_SHARD_SIZE as u16 >= config.shard_size,
				Error::<T>::ShardSizeAboveMax
			);
			ensure!(
				config.shard_size >= config.shard_threshold,
				Error::<T>::ThresholdLargerThanSize
			);
			NetworkBatchSize::<T>::insert(network, config.batch_size);
			NetworkBatchOffset::<T>::insert(network, config.batch_offset);
			NetworkBatchGasLimit::<T>::insert(network, config.batch_gas_limit);
			NetworkShardTaskLimit::<T>::insert(network, config.shard_task_limit);
			NetworkShardSize::<T>::insert(network, config.shard_size);
			NetworkShardThreshold::<T>::insert(network, config.shard_threshold);
			Self::deposit_event(Event::NetworkConfigChanged(network, config));
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
		///    2. Call `Self::insert_network(network).
		///    3. Emit the [`Event::NetworkRegistered`] event with the new `NetworkId`.
		///    4. Return `Ok(())` to indicate success.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register_network(network.chain_name.0.len() as u32, network.chain_network.0.len() as u32))]
		pub fn register_network(origin: OriginFor<T>, network: Network) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Self::insert_network(&network)?;
			Ok(())
		}

		/// Sets the configuration for a specific network.
		///
		/// # Flow
		///   1. Ensure the origin of the transaction is a root user.
		///   2. Insert the new batch size for the specified network into the [`NetworkBatchSize`] storage.
		///   3. Insert the new offset for the specified network into the [`NetworkBatchOffset`] storage.
		///   4. Emit an event indicating the batch size and offset have been set.
		///   5. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::set_network_config())]
		pub fn set_network_config(
			origin: OriginFor<T>,
			network: NetworkId,
			config: NetworkConfig,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Self::insert_network_config(network, config)?;
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_network())]
		pub fn remove_network(origin: OriginFor<T>, network: NetworkId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Networks::<T>::remove(network);
			NetworkName::<T>::remove(network);
			NetworkGatewayAddress::<T>::remove(network);
			NetworkGatewayBlock::<T>::remove(network);
			NetworkBatchSize::<T>::remove(network);
			NetworkBatchGasLimit::<T>::remove(network);
			NetworkShardTaskLimit::<T>::remove(network);
			NetworkShardSize::<T>::remove(network);
			NetworkShardThreshold::<T>::remove(network);
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
		fn get_networks() -> Vec<NetworkId> {
			NetworkName::<T>::iter().map(|(n, _)| n).collect()
		}

		fn gateway(network: NetworkId) -> Option<Address> {
			NetworkGatewayAddress::<T>::get(network)
		}

		fn next_batch_size(network: NetworkId, block_height: u64) -> u32 {
			let network_batch_size = NetworkBatchSize::<T>::get(network).unwrap_or(32);
			let network_offset = NetworkBatchOffset::<T>::get(network).unwrap_or_default();
			network_batch_size
				- ((block_height + network_offset as u64) % network_batch_size as u64) as u32
		}

		fn batch_gas_limit(network: NetworkId) -> u128 {
			NetworkBatchGasLimit::<T>::get(network).unwrap_or(10_000)
		}

		fn shard_task_limit(network: NetworkId) -> u32 {
			NetworkShardTaskLimit::<T>::get(network).unwrap_or(10)
		}

		fn shard_size(network: NetworkId) -> u16 {
			NetworkShardSize::<T>::get(network).unwrap_or(3)
		}

		fn shard_threshold(network: NetworkId) -> u16 {
			NetworkShardThreshold::<T>::get(network).unwrap_or(2)
		}
	}
}
