#![cfg_attr(not(feature = "std"), no_std)]
//! # Elections Pallet
//!
//!
//!
//! The flowchart represents the logical flow and interactions within the pallet,
//! detailing how various functions and operations are interconnected. It begins
//! with different entry points corresponding to various operations: setting
//! shard configuration, setting electable members, handling member online/offline
//! events, handling shard offline events.
//!
//! **Set Shard Configuration** Flow starts by ensuring the caller is a root user,
//! validating the shard size and threshold, updating the storage, emitting an
//! event, iterating through unassigned members, and trying to elect a new shard.
//!
//! **Member Online** Flow checks if the member is part of a shard. If not, it
//! verifies if the member is electable, adds them to the unassigned list, and
//! attempts to elect a new shard. If the member is already part of a shard, it
//! simply notifies the shards interface.
//!
//! **Shard Offline** Flow adds the shard members to the unassigned list and tries
//! to elect a new shard.
//!
//! **Try Elect Shard** Flow evaluates if a new shard can be formed, removes the
//! selected members from the unassigned list, and creates a new shard using the
//! shards interface.
//!
//! **New Shard Members** Flow retrieves the required shard size, gathers
//! unassigned and online members, ensures there are enough members to form a
//! shard, sorts members by stake, selects the top members to form the shard, and
//! returns the selected members.
//!
#![doc = simple_mermaid::mermaid!("../docs/elections_flow.mmd")]
//!
#![doc = simple_mermaid::mermaid!("../docs/elections_flow_2.mmd")]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use polkadot_sdk::{frame_support, frame_system, sp_runtime, sp_std};

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::Saturating;
	use sp_std::vec;
	use sp_std::vec::Vec;

	use time_primitives::{
		AccountId, ElectionsInterface, MembersInterface, NetworkId, ShardsInterface, MAX_SHARD_SIZE,
	};

	pub trait WeightInfo {
		fn set_shard_config() -> Weight;
		fn try_elect_shard(b: u32) -> Weight;
	}

	impl WeightInfo for () {
		fn set_shard_config() -> Weight {
			Weight::default()
		}
		fn try_elect_shard(_: u32) -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]

	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config<AccountId = AccountId> {
		/// The runtime event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		///  The weight information for the pallet's extrinsics.
		type WeightInfo: WeightInfo;
		/// Ensured origin for calls changing config or electables
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// The interface for shard-related operations.
		type Shards: ShardsInterface;
		///  The storage interface for member-related data.
		type Members: MembersInterface;
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

	/// Electable shard members
	#[pallet::storage]
	pub type Electable<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, AccountId, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		#[serde(skip)]
		pub _p: PhantomData<T>,
		pub shard_size: u16,
		pub shard_threshold: u16,
		pub electable: Vec<AccountId>,
	}

	impl<T> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				_p: PhantomData,
				shard_size: 3,
				shard_threshold: 2,
				electable: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			assert!(
				MAX_SHARD_SIZE >= self.shard_size.into(),
				"Attempted to set ShardSize {} > MaxShardSize {} which bounds commitment length",
				self.shard_size,
				MAX_SHARD_SIZE
			);
			ShardSize::<T>::put(self.shard_size);
			ShardThreshold::<T>::put(self.shard_threshold);
			for account in &self.electable {
				Electable::<T>::insert(account.clone(), account);
			}
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
		ShardSizeAboveMax,
		ThresholdLargerThanSize,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			log::info!("on_initialize begin");
			let mut weight = Weight::default();
			Unassigned::<T>::iter().map(|(n, _, _)| n).for_each(|network| {
				weight = weight.saturating_add(Self::try_elect_shard(network));
			});
			log::info!("on_initialize end");
			weight
		}
	}

	/// The pallet provides mechanisms to configure and manage shards, elect members to shards,
	/// and handle member events. It includes storage items to keep track of shard sizes,
	/// thresholds, and electable members. It also defines events and errors related to shard
	/// management.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		///  Sets the configuration for shard size and threshold.
		/// # Flow:
		///    1. Ensures the caller is the root.
		///    2. Validates that `shard_size` is greater than or equal to `shard_threshold`.
		///    3. Updates [`ShardSize`] and [`ShardThreshold`] storage values.
		///    4. Emits the [`Event::ShardConfigSet`] event.
		///    5. Iterates through all unassigned members in the [`Unassigned`] storage.
		///    6. Calls `try_elect_shard` for each network to form new shards if possible.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_shard_config())]
		pub fn set_shard_config(
			origin: OriginFor<T>,
			shard_size: u16,
			shard_threshold: u16,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			ensure!(MAX_SHARD_SIZE >= shard_size.into(), Error::<T>::ShardSizeAboveMax);
			ensure!(shard_size >= shard_threshold, Error::<T>::ThresholdLargerThanSize);
			ShardSize::<T>::put(shard_size);
			ShardThreshold::<T>::put(shard_threshold);
			Self::deposit_event(Event::ShardConfigSet(shard_size, shard_threshold));
			for (network, _, _) in Unassigned::<T>::iter() {
				Self::try_elect_shard(network);
			}
			Ok(())
		}

		///  Sets the electable members who can be assigned to shards.
		/// # Flow
		///    1. Ensures the caller is the root.
		///    2. Clears the existing electable members list from the [`Electable`] storage.
		///    3. Inserts the new electable members into the [`Electable`] storage.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_shard_config())]
		pub fn set_electable(origin: OriginFor<T>, electable: Vec<AccountId>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let _ = Electable::<T>::clear(u32::MAX, None);
			for account in electable {
				Electable::<T>::insert(account.clone(), account);
			}
			Ok(())
		}
	}

	impl<T: Config> ElectionsInterface for Pallet<T> {
		///  Handles the event when a shard goes offline.
		/// # Flow
		///    1. Inserts each member of the offline shard into the [`Unassigned`] storage for the given network.
		fn shard_offline(network: NetworkId, members: Vec<AccountId>) {
			members.into_iter().for_each(|m| Unassigned::<T>::insert(network, m, ()));
		}

		///  Retrieves the default shard size.
		/// # Flow
		///    1. Returns the value of [`ShardSize`] from storage.
		fn default_shard_size() -> u16 {
			ShardSize::<T>::get()
		}

		///  Handles the event when a member comes online.
		/// # Flow
		///    1. Checks if the member is not already a shard member.
		///    2. Checks if the member is electable or if there are no electable members defined.
		///    3. Inserts the member into the [`Unassigned`] storage for the given network.
		///    4. Notifies the `Shards` interface about the member coming online.
		fn member_online(member: &AccountId, network: NetworkId) {
			if !T::Shards::is_shard_member(member)
				&& (Electable::<T>::iter().next().is_none()
					|| Electable::<T>::get(member).is_some())
			{
				Unassigned::<T>::insert(network, member, ());
			}
			T::Shards::member_online(member, network);
		}
		///   Handles the event when a member goes offline.
		/// # Flow
		///    1. Removes the member from the [`Unassigned`] storage for the given network.
		///    2. Notifies the `Shards` interface about the member going offline.
		///    3. Returns the weight of the operation.
		fn member_offline(member: &AccountId, network: NetworkId) {
			Unassigned::<T>::remove(network, member);
			T::Shards::member_offline(member, network);
		}
	}

	impl<T: Config> Pallet<T> {
		/// Return electable accounts.
		pub fn get_electable() -> Vec<AccountId> {
			Electable::<T>::iter().map(|(_, e)| e).collect()
		}

		///   Attempts to elect a new shard for a network.
		/// # Flow
		///    1. Calls `new_shard_members` to get a list of new shard members.
		///    2. If a new shard can be formed, removes the selected members from [`Unassigned`] storage.
		///    3. Creates a new shard using the `Shards` interface with the selected members and current shard threshold.
		pub(crate) fn try_elect_shard(network: NetworkId) -> Weight {
			let num_unassigned = match Self::new_shard_members(network) {
				(Some(members), n) => {
					members.iter().for_each(|m| Unassigned::<T>::remove(network, m));
					T::Shards::create_shard(network, members, ShardThreshold::<T>::get());
					n
				},
				(None, n) => n,
			};
			T::WeightInfo::try_elect_shard(num_unassigned)
		}

		///  Determines the members for a new shard.
		/// # Flow
		///    1. Retrieves the required shard size.
		///    2. Collects unassigned members for the given network who are online.
		///    3. Returns `None` if there are not enough members to form a shard.
		///    4. If there are just enough members, returns the list of members.
		///    5. If there are more members than needed, sorts them by their stake and selects the top members to form the shard.
		fn new_shard_members(network: NetworkId) -> (Option<Vec<AccountId>>, u32) {
			let mut num_unassigned: u32 = 0;
			let mut members = Vec::new();
			for (m, _) in Unassigned::<T>::iter_prefix(network) {
				if T::Members::is_member_online(&m) {
					members.push(m);
				}
				num_unassigned = num_unassigned.saturating_plus_one();
			}
			let shard_members_len = ShardSize::<T>::get() as usize;
			if members.len() < shard_members_len {
				return (None, num_unassigned);
			}
			if members.len() == shard_members_len {
				return (Some(members), num_unassigned);
			}
			// else members.len() > shard_members_len:
			members.sort_unstable_by(|a, b| {
				T::Members::member_stake(a)
					.cmp(&T::Members::member_stake(b))
					// sort by AccountId iff amounts are equal to uphold determinism
					.then_with(|| a.cmp(b))
					.reverse()
			});
			(Some(members.into_iter().take(shard_members_len).collect()), num_unassigned)
		}
	}
}
