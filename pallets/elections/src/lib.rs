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
	use sp_std::vec::Vec;
	use time_primitives::{
		AccountId, ElectionsInterface, MemberEvents, MemberStorage, Network, ShardsInterface,
	};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type Shards: ShardsInterface + MemberEvents;
		type Members: MemberStorage;
		#[pallet::constant]
		type ShardSize: Get<u16>;
		#[pallet::constant]
		type Threshold: Get<u16>;
	}

	/// Unassigned members per network
	#[pallet::storage]
	pub type Unassigned<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		Network,
		Blake2_128Concat,
		AccountId,
		(),
		OptionQuery,
	>;

	impl<T: Config> MemberEvents for Pallet<T> {
		fn member_online(member: &AccountId, network: Network) {
			if !T::Shards::is_shard_member(&member) {
				Unassigned::<T>::insert(network, member, ());
				Self::try_elect_shard(network);
			}
			T::Shards::member_online(member, network);
		}
		fn member_offline(member: &AccountId, network: Network) {
			Unassigned::<T>::remove(network, member);
			T::Shards::member_offline(member, network);
		}
	}

	impl<T: Config> ElectionsInterface for Pallet<T> {
		fn shard_offline(network: Network, members: Vec<AccountId>) {
			for member in members {
				Unassigned::<T>::insert(network, member, ());
			}
			Self::try_elect_shard(network);
		}
	}

	impl<T: Config> Pallet<T> {
		fn try_elect_shard(n: Network) {
			if let Some(m) = Self::new_shard_members(T::ShardSize::get() as usize, n) {
				T::Shards::create_shard(n, m, T::Threshold::get());
			}
		}

		fn new_shard_members(n: usize, network: Network) -> Option<Vec<AccountId>> {
			let members = Unassigned::<T>::iter_prefix(network)
				.map(|(m, _)| m)
				.filter(|m| T::Members::is_member_online(&m))
				.take(n)
				.collect::<Vec<_>>();
			if members.len() == n {
				Some(members)
			} else {
				None
			}
		}
	}
}
