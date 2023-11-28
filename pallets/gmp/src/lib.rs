#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use alloy_sol_types::SolCall;
	use codec::alloc::string::ToString;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use time_primitives::{
		registerTSSKeysCall, AccountId, Function, GmpInterface, MakeTask, Network, ShardId,
		ShardsEvents, ShardsInterface, TaskDescriptorParams, TssKey,
	};

	pub trait WeightInfo {
		fn deploy_gateway() -> Weight;
		fn register_shards() -> Weight;
		fn revoke_shards() -> Weight;
	}

	impl WeightInfo for () {
		fn deploy_gateway() -> Weight {
			Weight::default()
		}
		fn register_shards() -> Weight {
			Weight::default()
		}
		fn revoke_shards() -> Weight {
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

	/// Address for gateway contracts deployed by network
	#[pallet::storage]
	pub type GatewayAddress<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, [u8; 20], OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		NoNetworkForChainID,
		GatewayNotDeployed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		GatewayContractDeployed(Network, [u8; 20]),
		ShardsRegistered(Network),
		ShardsRevoked(Network),
		Executed(Network),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::deploy_gateway())]
		pub fn deploy_gateway(
			origin: OriginFor<T>,
			chain_id: u64,
			contract_address: [u8; 20],
			shard_ids: Vec<ShardId>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let network = chain_id.try_into().map_err(|_| Error::<T>::NoNetworkForChainID)?;
			GatewayAddress::<T>::insert(network, contract_address);
			for shard in shard_ids {
				ShardRegistry::<T>::insert(shard, network, ());
			}
			Self::deposit_event(Event::GatewayContractDeployed(network, contract_address));
			Ok(())
		}
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::register_shards())]
		pub fn register_shards(
			origin: OriginFor<T>,
			chain_id: u64,
			shard_ids: Vec<ShardId>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let network = chain_id.try_into().map_err(|_| Error::<T>::NoNetworkForChainID)?;
			ensure!(GatewayAddress::<T>::get(network).is_some(), Error::<T>::GatewayNotDeployed);
			for shard in shard_ids {
				ShardRegistry::<T>::insert(shard, network, ());
			}
			Self::deposit_event(Event::ShardsRegistered(network));
			Ok(())
		}
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::revoke_shards())]
		pub fn revoke_shards(
			origin: OriginFor<T>,
			chain_id: u64,
			shard_ids: Vec<ShardId>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let network = chain_id.try_into().map_err(|_| Error::<T>::NoNetworkForChainID)?;
			ensure!(GatewayAddress::<T>::get(network).is_some(), Error::<T>::GatewayNotDeployed);
			for shard in shard_ids {
				ShardRegistry::<T>::remove(shard, network);
			}
			Self::deposit_event(Event::ShardsRevoked(network));
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
			let Some(contract_address) = GatewayAddress::<T>::get(network) else {
				return None;
			};
			let Some(shard_public_key) = T::Shards::tss_public_key(shard_id) else {
				return None;
			};
			let mut key_vec = shard_public_key.to_vec();
			let x_coordinate: [u8; 32] = shard_public_key[1..].try_into().unwrap();
			let (parity, coord_x) = (key_vec.remove(0), x_coordinate.into());
			Some(Function::SendMessage {
				contract_address: contract_address.to_vec(),
				payload: registerTSSKeysCall {
					// signed at execution => dummy signature now
					signature: Default::default(),
					tssKeys: sp_std::vec![TssKey { parity, coordX: coord_x }],
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
