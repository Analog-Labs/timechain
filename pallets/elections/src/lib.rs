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

	impl<T: Config> ElectionsInterface for Pallet<T> {
		fn unassigned_member_online(network: Network) {
			if let Some(members) =
				T::Members::new_shard_members(T::ShardSize::get() as usize, network)
			{
				T::Shards::create_shard(network, members, T::Threshold::get());
			}
		}
	}
}
