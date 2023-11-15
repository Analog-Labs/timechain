#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

pub mod sol;
pub use sol::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::IGateway::*;
	use alloy_sol_types::SolCall;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use time_primitives::{
		AccountId, Function, Gateway, GmpInterface, Network, ShardId, ShardsInterface,
	};

	pub trait WeightInfo {
		fn deploy_gateway() -> Weight;
	}

	impl WeightInfo for () {
		fn deploy_gateway() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Shards: ShardsInterface;
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
	/// TODO: change to just value contract_address once Network includes chain_id
	#[pallet::storage]
	pub type Gateways<T: Config> = StorageMap<_, Blake2_128Concat, Network, Gateway, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		GatewayContractDeployed(Network, [u8; 20]), //TODO make call args event args
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::deploy_gateway())]
		pub fn deploy_gateway(
			origin: OriginFor<T>,
			network: Network,
			chain_id: u64,
			contract_address: [u8; 20],
			shard_ids: Vec<ShardId>,
		) -> DispatchResult {
			ensure_root(origin)?;
			Gateways::<T>::insert(
				network,
				Gateway {
					address: contract_address.clone().to_vec(),
					chain_id,
				},
			);
			for shard in shard_ids {
				Self::register_shard(shard, network);
			}
			Self::deposit_event(Event::GatewayContractDeployed(network, contract_address));
			Ok(())
		}
	}

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
		fn register_shard(shard_id: ShardId, network: Network) {
			ShardRegistry::<T>::insert(shard_id, network, ());
		}
		/// Callback for scheduling a task to register the shard for the network
		fn schedule_register_shard(shard_id: ShardId, network: Network) {
			RegisterShardScheduled::<T>::insert(shard_id, network, ());
		}
		/// Return Some(Function) if gateway contract deployed on network
		fn get_register_shard(shard_id: ShardId, network: Network) -> Option<Function> {
			let Some(gateway) = Gateways::<T>::get(network) else {
				return None;
			};
			let Some(shard_public_key) = T::Shards::tss_public_key(shard_id) else {
				return None;
			};
			Some(Function::SendMessage {
				contract_address: gateway.address,
				payload: registerTSSKeysCall {
					signature: Default::default(),
					tssKeys: shard_public_key.as_slice().to_vec(),
				}
				.abi_encode(),
			})
		}
	}
}
