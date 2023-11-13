#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use time_primitives::{
		AccountId, Function, Gateway, GmpInterface, Network, ShardId, TssSignature,
	};

	pub trait WeightInfo {
		fn send_message() -> Weight;
	}

	impl WeightInfo for () {
		fn send_message() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type WeightInfo: WeightInfo;
	}

	/// Where or not a register shard task has been scheduled for a Network
	#[pallet::storage]
	pub type RegisterShardScheduled<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, Network, (), OptionQuery>;

	/// Whether or not a shard is registered to be assigned write tasks on a ChainId
	#[pallet::storage]
	pub type ShardRegistry<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, Network, (), OptionQuery>;

	/// Gateway contracts deployed on each network
	#[pallet::storage]
	pub type Gateways<T: Config> = StorageMap<_, Blake2_128Concat, Network, Gateway, OptionQuery>;

	impl<T: Config> GmpInterface for Pallet<T> {
		/// Return true if shard is registered to perform write tasks for the network
		fn is_shard_registered(shard_id: ShardId, network: Network) -> bool {
			ShardRegistry::<T>::get(shard_id, network).is_some()
		}
		/// Returns true if register shard is scheduled for the network
		fn is_register_shard_scheduled(shard_id: ShardId, network: Network) -> bool {
			RegisterShardScheduled::<T>::get(shard_id, network).is_some()
		}
		/// Callback for registering the shard for the network
		fn registered_shard(shard_id: ShardId, network: Network) {
			ShardRegistry::<T>::insert(shard_id, network, ());
		}
		/// Callback for scheduling a task to register the shard for the network
		fn scheduled_register_shard(shard_id: ShardId, network: Network) {
			RegisterShardScheduled::<T>::insert(shard_id, network, ());
		}
		/// Return Some(Function) if gateway contract deployed on network
		fn register_shard_call(shard_id: ShardId, network: Network) -> Option<Function> {
			let Some(gateway) = Gateways::<T>::get(network) else {
				// contract not deployed (or storage not updated post deploy)
				return None;
			};
			Some(Function::SendMessage {
				contract_address: gateway.address,
				// use gateway.chain_id to form register_shard call to submit function
				// on gateway contract
				// function selector: 0xb58145f3
				//  args: shardid: u64, bytes: Vec<u8>, TssSignature signature
				// TODO: where does signature come from
				// bytes = RegisterShard(shardid, pubkey)
				payload: Vec::new(),
			})
		}
	}
}
