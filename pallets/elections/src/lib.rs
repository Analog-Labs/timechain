#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]
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
		ShardsInterface,
	};

	pub trait WeightInfo {
		fn try_elect_shards(b: u32) -> Weight;
	}

	impl WeightInfo for () {
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

	/// Unassigned online members per network sorted by stake and then AccountId
	#[pallet::storage]
	pub type Unassigned<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, Vec<AccountId>, ValueQuery>;

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			log::info!("on_initialize begin");
			let mut num_elections = 0u32;
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
			T::WeightInfo::try_elect_shards(num_elections)
		}
	}

	impl<T: Config> ElectionsInterface for Pallet<T> {
		type MaxElectionsPerBlock = T::MaxElectionsPerBlock;
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

		///   Handles the event when members go offline.
		/// # Flow
		///    1. Removes the members from the [`Unassigned`] storage for the given network.
		///    2. Notifies the `Shards` interface about the member going offline.
		///    3. Returns the weight of the operation.
		fn members_offline(members: Vec<AccountId>, network: NetworkId) {
			Unassigned::<T>::mutate(network, |unassigned| {
				unassigned.retain(|m| !members.contains(m));
			});
			T::Shards::members_offline(members);
		}
	}

	impl<T: Config> Pallet<T> {
		/// Elects as many as `max_elections` number of new shards for `networks`
		/// Returns # of Shards Elected
		pub(crate) fn try_elect_shards(network: NetworkId, max_elections: u32) -> u32 {
			let shard_size = T::Networks::shard_size(network);
			let shard_threshold = T::Networks::shard_threshold(network);
			let mut unassigned = Unassigned::<T>::get(network);
			let num_elected =
				sp_std::cmp::min((unassigned.len() as u32) / shard_size as u32, max_elections)
					* shard_size as u32;
			let mut members = Vec::with_capacity(num_elected as usize);
			members.extend(unassigned.drain(..(num_elected as usize)));
			let mut num_elections = 0u32;
			for (i, next_shard) in members.chunks(shard_size as usize).enumerate() {
				if T::Shards::create_shard(network, next_shard.to_vec(), shard_threshold).is_err() {
					unassigned
						.extend(members.chunks(shard_size as usize).skip(i).flatten().cloned());
					break;
				} else {
					num_elections += 1;
				}
			}
			Unassigned::<T>::insert(network, unassigned);
			num_elections
		}
	}
}
