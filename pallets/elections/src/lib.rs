#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use sp_runtime::traits::Saturating;
	use time_primitives::{AccountId, ElectionsInterface, MemberElections, Network, ShardCreator};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type Shards: ShardCreator;
		type Members: MemberElections;
		#[pallet::constant]
		type ShardSize: Get<u8>;
		#[pallet::constant]
		type Threshold: Get<u16>;
	}

	/// Counter to determine when shard should be created
	#[pallet::storage]
	pub type OnlineUnassigned<T: Config> = StorageMap<_, Blake2_128Concat, Network, u8, ValueQuery>;

	impl<T: Config> ElectionsInterface for Pallet<T> {
		fn member_online(network: Network) {
			let online_unassigned = OnlineUnassigned::<T>::get(network).saturating_plus_one();
			if online_unassigned == T::ShardSize::get() {
				T::Shards::create_shard(
					network,
					T::Members::get_unassigned_members(T::ShardSize::get() as usize, network),
					T::Threshold::get(),
				);
				OnlineUnassigned::<T>::insert(network, 0);
			} else {
				OnlineUnassigned::<T>::insert(network, online_unassigned);
			}
		}
	}
}
