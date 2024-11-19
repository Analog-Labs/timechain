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
	use polkadot_sdk::{frame_support, frame_system, sp_std};

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_std::vec;
	use sp_std::vec::Vec;

	use time_primitives::{
		AccountId, ElectionsInterface, MembersInterface, NetworkId, NetworksInterface,
		ShardsInterface, MAX_SHARD_SIZE,
	};

	pub trait WeightInfo {
		fn set_shard_config() -> Weight;
		fn try_elect_shards(b: u32) -> Weight;
	}

	impl WeightInfo for () {
		fn set_shard_config() -> Weight {
			Weight::default()
		}
		fn try_elect_shards(_: u32) -> Weight {
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
		/// The networks interface for getting all networks
		type Networks: NetworksInterface;
		/// Maximum number of shard elections per block
		#[pallet::constant]
		type MaxElectionsPerBlock: Get<u32>;
	}

	/// Counter for electing shards per network in order over multiple blocks
	#[pallet::storage]
	pub type NetworkCounter<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Size of each new shard
	#[pallet::storage]
	pub type ShardSize<T: Config> = StorageValue<_, u16, ValueQuery>;

	/// Threshold of each new shard
	#[pallet::storage]
	pub type ShardThreshold<T: Config> = StorageValue<_, u16, ValueQuery>;

	/// Unassigned online members per network sorted by stake and then AccountId
	#[pallet::storage]
	pub type Unassigned<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, Vec<AccountId>, ValueQuery>;

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
			assert!(
				MAX_SHARD_SIZE >= self.shard_size.into(),
				"Attempted to set ShardSize {} > MaxShardSize {} which bounds commitment length",
				self.shard_size,
				MAX_SHARD_SIZE
			);
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
		ShardSizeAboveMax,
		ThresholdLargerThanSize,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			log::info!("on_initialize begin");
			let (mut weight, mut num_elections) = (Weight::default(), 0u32);
			let networks = T::Networks::get_networks();
			let net_counter0 = NetworkCounter::<T>::get();
			let (mut net_counter, mut all_nets_elected) = (net_counter0, false);
			while num_elections < T::MaxElectionsPerBlock::get() {
				let Some(next_network) = networks.get(net_counter as usize) else {
					net_counter = 0;
					break;
				};
				let elected = Self::try_elect_shards(
					*next_network,
					T::MaxElectionsPerBlock::get().saturating_sub(num_elections),
				);
				weight = weight.saturating_add(T::WeightInfo::try_elect_shards(elected));
				num_elections = num_elections.saturating_add(elected);
				net_counter = (net_counter + 1) % networks.len() as u32;
				if net_counter == net_counter0 {
					all_nets_elected = true;
					break;
				}
			}
			if !all_nets_elected {
				NetworkCounter::<T>::put(net_counter);
			} // else counter starts where it left off => no write required
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
			for network in T::Networks::get_networks() {
				Self::try_elect_shards(network, T::MaxElectionsPerBlock::get());
			}
			Ok(())
		}
	}

	impl<T: Config> ElectionsInterface for Pallet<T> {
		///  Handles the event when a shard goes offline.
		/// # Flow
		///    1. Inserts each member of the offline shard into the [`Unassigned`] storage for the given network.
		fn shard_offline(network: NetworkId, members: Vec<AccountId>) {
			let mut batch = Vec::new();
			for member in members {
				if !T::Members::is_member_registered(&member) {
					T::Members::unstake_member(&member);
				} else if T::Members::is_member_online(&member) {
					batch.push(member.clone());
				}
			}
			Unassigned::<T>::mutate(network, |unassigned| {
				unassigned.extend(batch);
				unassigned.sort_by(|a, b| {
					T::Members::member_stake(a)
						.cmp(&T::Members::member_stake(b))
						// sort by AccountId iff amounts are equal to uphold determinism
						.then_with(|| a.cmp(b))
						.reverse()
				});
			});
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
			if !T::Shards::is_shard_member(member) {
				Unassigned::<T>::mutate(network, |members| {
					members.push(member.clone());
					members.sort_by(|a, b| {
						T::Members::member_stake(a)
							.cmp(&T::Members::member_stake(b))
							// sort by AccountId iff amounts are equal to uphold determinism
							.then_with(|| a.cmp(b))
							.reverse()
					});
				});
			}
			T::Shards::member_online(member, network);
		}
		///   Handles the event when a member goes offline.
		/// # Flow
		///    1. Removes the member from the [`Unassigned`] storage for the given network.
		///    2. Notifies the `Shards` interface about the member going offline.
		///    3. Returns the weight of the operation.
		fn member_offline(member: &AccountId, network: NetworkId) {
			Unassigned::<T>::mutate(network, |members| {
				members.retain(|m| m != member);
			});
			T::Shards::member_offline(member, network);
		}
	}

	impl<T: Config> Pallet<T> {
		/// Elects as many as `max_elections` number of new shards for `networks`
		/// Returns # of Shards Elected
		pub(crate) fn try_elect_shards(network: NetworkId, max_elections: u32) -> u32 {
			let shard_size = ShardSize::<T>::get();
			let mut unassigned = Unassigned::<T>::get(network);
			let num_elected = sp_std::cmp::min(
				(unassigned.len() as u32).saturating_div(shard_size.into()),
				max_elections,
			)
			.saturating_mul(shard_size.into());
			let mut members: Vec<AccountId> = unassigned.drain(..(num_elected as usize)).collect();
			Unassigned::<T>::insert(network, unassigned);
			let shard_threshold = ShardThreshold::<T>::get();
			let mut num_elections = 0u32;
			while !members.is_empty() {
				let next_shard: Vec<AccountId> = members.drain(..(shard_size as usize)).collect();
				T::Shards::create_shard(network, next_shard, shard_threshold);
				num_elections += 1;
			}
			num_elections
		}
	}
}
