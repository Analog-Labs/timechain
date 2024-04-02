#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::traits::BuildGenesisConfig;
	use frame_system::pallet_prelude::*;
	use sp_std::marker::PhantomData;
	use sp_std::vec::Vec;
	use time_primitives::{
		AccountId, ElectionsInterface, MemberEvents, MemberStorage, NetworkId, ShardsInterface,
	};

	pub trait WeightInfo {
		fn set_shard_config() -> Weight;
	}

	impl WeightInfo for () {
		fn set_shard_config() -> Weight {
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
		type Shards: ShardsInterface + MemberEvents;
		type Members: MemberStorage;
	}

	/// Size of each new shard
	#[pallet::storage]
	pub type ShardSize<T: Config> = StorageValue<_, u16, ValueQuery>;

	/// Threshold of each new shard
	#[pallet::storage]
	pub type ShardThreshold<T: Config> = StorageValue<_, u16, ValueQuery>;

	/// Unassigned members per network
	#[pallet::storage]
	pub type Unassigned<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		NetworkId,
		Blake2_128Concat,
		AccountId,
		(),
		OptionQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		#[serde(skip)]
		pub _p: PhantomData<T>,
		pub shard_size: u16,
		pub shard_threshold: u16,
	}

	impl<T> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				_p: PhantomData,
				shard_size: 3,
				shard_threshold: 2,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			ShardSize::<T>::put(self.shard_size);
			ShardThreshold::<T>::put(self.shard_threshold);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Set shard config: size, threshold
		ShardConfigSet(u16, u16),
	}

	#[pallet::error]
	pub enum Error<T> {
		ThresholdLargerThanSize,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_shard_config())]
		pub fn set_shard_config(
			origin: OriginFor<T>,
			shard_size: u16,
			shard_threshold: u16,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(shard_size >= shard_threshold, Error::<T>::ThresholdLargerThanSize);
			ShardSize::<T>::put(shard_size);
			ShardThreshold::<T>::put(shard_threshold);
			Self::deposit_event(Event::ShardConfigSet(shard_size, shard_threshold));
			Ok(())
		}
	}

	impl<T: Config> MemberEvents for Pallet<T> {
		fn member_online(member: &AccountId, network: NetworkId) {
			if !T::Shards::is_shard_member(member) {
				Unassigned::<T>::insert(network, member, ());
				Self::try_elect_shard(network);
			}
			T::Shards::member_online(member, network);
		}
		fn member_offline(member: &AccountId, network: NetworkId) {
			Unassigned::<T>::remove(network, member);
			T::Shards::member_offline(member, network);
		}
	}

	impl<T: Config> ElectionsInterface for Pallet<T> {
		fn shard_offline(network: NetworkId, members: Vec<AccountId>) {
			members.into_iter().for_each(|m| Unassigned::<T>::insert(network, m, ()));
			Self::try_elect_shard(network);
		}

		fn default_shard_size() -> u16 {
			ShardSize::<T>::get()
		}
	}

	impl<T: Config> Pallet<T> {
		fn try_elect_shard(network: NetworkId) {
			if let Some(members) = Self::new_shard_members(network) {
				members.iter().for_each(|m| Unassigned::<T>::remove(network, m));
				T::Shards::create_shard(network, members, ShardThreshold::<T>::get());
			}
		}

		fn new_shard_members(network: NetworkId) -> Option<Vec<AccountId>> {
			let shard_members_len = ShardSize::<T>::get() as usize;
			let mut members = Unassigned::<T>::iter_prefix(network)
				.map(|(m, _)| m)
				.filter(T::Members::is_member_online)
				.collect::<Vec<_>>();
			if members.len() < shard_members_len {
				return None;
			}
			if members.len() == shard_members_len {
				return Some(members);
			}
			// else members.len() > shard_members_len:
			members.sort_unstable_by(|a, b| {
				T::Members::member_stake(a)
					.cmp(&T::Members::member_stake(b))
					// sort by AccountId iff amounts are equal to uphold determinism
					.then_with(|| a.cmp(b))
					.reverse()
			});
			Some(members.into_iter().take(shard_members_len).collect())
		}
	}
}
