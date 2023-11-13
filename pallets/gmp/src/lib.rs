#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

// include information for
// (1) registering the shard
// (2) forming the submit (RegisterShard) call and calling it

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
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MessageSent(),
	}

	#[pallet::error]
	pub enum Error<T> {
		GatewayContractNotDeployed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::send_message())]
		pub fn send_message(
			origin: OriginFor<T>,
			_message: Vec<u8>,
			_inputs: TssSignature,
		) -> DispatchResult {
			ensure_signed(origin)?;
			// inherit pallet_evm through a trait or directly
			// TODO: eth.call_smart_contract()
			Self::deposit_event(Event::MessageSent());
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
		fn registered_shard(shard_id: ShardId, network: Network) {
			ShardRegistry::<T>::insert(shard_id, network, ());
		}
		/// Callback for scheduling a task to register the shard for the network
		fn scheduled_register_shard(shard_id: ShardId, network: Network) {
			RegisterShardScheduled::<T>::insert(shard_id, network, ());
		}
		fn register_shard_call(network: Network) -> Option<Function> {
			let Some(gateway) = Gateways::<T>::get(network) else {
				// contract not deployed (or storage not updated post deploy)
				return None;
			};
			Some(Function::SendMessage {
				contract_address: gateway.address,
				// use gateway.chain_id to form register_shard call to submit function
				// on gateway contract
				payload: Vec::new(),
			})
		}
	}
}
