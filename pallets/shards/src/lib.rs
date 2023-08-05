#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::{ValueQuery, *};
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use time_primitives::{
		Network, ScheduleInterface, ShardId, ShardInterface, TimeId, TssPublicKey,
	};

	pub trait WeightInfo {
		fn register_shard() -> Weight;
		fn submit_tss_public_key() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = sp_runtime::AccountId32> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type TaskScheduler: ScheduleInterface;
	}

	#[pallet::storage]
	#[pallet::getter(fn shard_id)]
	/// Counter for creating unique shard_ids during on-chain creation
	pub type ShardIdCounter<T: Config> = StorageValue<_, ShardId, ValueQuery>;

	/// Network for which shards can be assigned tasks
	#[pallet::storage]
	#[pallet::getter(fn shard_network)]
	pub type ShardNetwork<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, Network, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_public_key)]
	pub type ShardPublicKey<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, TssPublicKey, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_collector)]
	pub type ShardCollector<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, TimeId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_members)]
	pub type ShardMembers<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TimeId, (), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New shard was created
		ShardCreated(ShardId, Network),
		/// Shard completed dkg and submitted public key to runtime
		ShardOnline(ShardId, TssPublicKey),
	}

	#[pallet::error]
	pub enum Error<T> {
		UnknownShard,
		PublicKeyAlreadyRegistered,
		InvalidNumberOfShardMembers,
		NotSignedByCollector,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Root can register new shard via providing
		/// set of IDs matching one of supported size of shard
		/// # Param
		/// * members - supported sized set of shard members Id
		/// * collector - index of collector if not index 0
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::register_shard())]
		pub fn register_shard(
			origin: OriginFor<T>,
			members: Vec<TimeId>,
			network: Network,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(members.len() == 3, Error::<T>::InvalidNumberOfShardMembers);
			let shard_id = <ShardIdCounter<T>>::get();
			<ShardIdCounter<T>>::put(shard_id + 1);
			<ShardNetwork<T>>::insert(shard_id, network);
			<ShardCollector<T>>::insert(shard_id, members[0].clone());
			for member in members {
				<ShardMembers<T>>::insert(shard_id, member, ());
			}
			Self::deposit_event(Event::ShardCreated(shard_id, network));
			Ok(())
		}

		/// Submits TSS group key to runtime
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::submit_tss_public_key())]
		pub fn submit_tss_public_key(
			origin: OriginFor<T>,
			shard_id: ShardId,
			public_key: TssPublicKey,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let collector = ShardCollector::<T>::get(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let network = ShardNetwork::<T>::get(shard_id).ok_or(Error::<T>::UnknownShard)?;
			ensure!(
				ShardPublicKey::<T>::get(shard_id).is_none(),
				Error::<T>::PublicKeyAlreadyRegistered
			);
			ensure!(account_id == collector, Error::<T>::NotSignedByCollector);
			<ShardPublicKey<T>>::insert(shard_id, public_key);
			Self::deposit_event(Event::ShardOnline(shard_id, public_key));
			T::TaskScheduler::shard_online(shard_id, network);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_shards(time_id: TimeId) -> Vec<ShardId> {
			ShardMembers::<T>::iter()
				.filter_map(
					|(shard_id, member, _)| {
						if member == time_id {
							Some(shard_id)
						} else {
							None
						}
					},
				)
				.collect()
		}

		pub fn get_shard_members(shard_id: ShardId) -> Vec<TimeId> {
			ShardMembers::<T>::iter_prefix(shard_id).map(|(time_id, _)| time_id).collect()
		}
	}

	impl<T: Config> ShardInterface for Pallet<T> {
		fn collector(shard_id: ShardId) -> Option<TimeId> {
			ShardCollector::<T>::get(shard_id)
		}
	}
}
