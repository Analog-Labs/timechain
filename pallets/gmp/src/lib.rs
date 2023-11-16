#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

pub mod sol;
pub use sol::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::{IGateway::*, *};
	use alloy_sol_types::SolCall;
	use codec::alloc::string::ToString;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use time_primitives::{
		AccountId, Function, Gateway, GmpInterface, MakeTask, Network, ShardId, ShardsEvents,
		ShardsInterface, TaskDescriptorParams,
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
		type Tasks: MakeTask;
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
				ShardRegistry::<T>::insert(shard, network, ());
			}
			Self::deposit_event(Event::GatewayContractDeployed(network, contract_address));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Return true if shard is registered to perform write tasks for the network
		fn is_shard_registered(shard_id: ShardId, network: Network) -> bool {
			ShardRegistry::<T>::get(shard_id, network).is_some()
		}
		/// Returns true if register shard is scheduled for the network
		fn is_register_shard_scheduled(shard_id: ShardId, network: Network) -> bool {
			RegisterShardScheduled::<T>::get(shard_id, network).is_some()
		}

		/// Return Some(Function) if gateway contract deployed on network
		fn register_shard_call(shard_id: ShardId, network: Network) -> Option<Function> {
			let Some(gateway) = Gateways::<T>::get(network) else {
				return None;
			};
			let Some(shard_public_key) = T::Shards::tss_public_key(shard_id) else {
				return None;
			};
			let mut key_vec = shard_public_key.to_vec();
			let x_coordinate: [u8; 32] = shard_public_key[1..].try_into().unwrap();
			let (parity, coord_x) = (key_vec.remove(0), x_coordinate.into());
			Some(Function::SendMessage {
				contract_address: gateway.address,
				payload: registerTSSKeysCall {
					// signed at execution => dummy signature now
					signature: Default::default(),
					tssKeys: vec![TssKey { parity, coordX: coord_x }],
				}
				.abi_encode(),
			})
		}
		/// Schedule register a shard
		fn schedule_register_shard(shard_id: ShardId, network: Network) {
			if !Self::is_shard_registered(shard_id, network)
				&& !Self::is_register_shard_scheduled(shard_id, network)
			{
				if let Some(function) = Self::register_shard_call(shard_id, network) {
					if T::Tasks::make_task(
						[0u8; 32].into(),
						TaskDescriptorParams {
							network,
							cycle: 0,
							start: 0,
							period: 1,
							hash: "".to_string(),
							function,
						},
					)
					.is_ok()
					{
						RegisterShardScheduled::<T>::insert(shard_id, network, ());
					} // TODO: handle error branch by logging something
				} // TODO: handle error branch by logging something
			}
		}
	}

	impl<T: Config> ShardsEvents for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: Network) {
			Self::schedule_register_shard(shard_id, network);
		}

		fn shard_offline(shard_id: ShardId, network: Network) {
			ShardRegistry::<T>::remove(shard_id, network);
		}
	}

	impl<T: Config> GmpInterface for Pallet<T> {
		fn task_assignable_to_shard(shard: ShardId, write: Option<Network>) -> bool {
			if let Some(network) = write {
				// only registered && online shards can do write tasks
				Self::is_shard_registered(shard, network) && T::Shards::is_shard_online(shard)
			} else {
				// any shard (registered or non-registered) can do read, sign tasks
				T::Shards::is_shard_online(shard)
			}
		}
	}
}
