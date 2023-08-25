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
	use frame_system::pallet_prelude::*;
	use strum::IntoEnumIterator;
	use time_primitives::{AccountId, MemberState, Network, PeerId};

	pub trait WeightInfo {
		fn join() -> Weight;
		fn leave() -> Weight;
	}

	impl WeightInfo for () {
		fn join() -> Weight {
			Weight::default()
		}
		fn leave() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type WeightInfo: WeightInfo;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		#[pallet::constant]
		type MinimumShard: Get<u8>;
		#[pallet::constant]
		type HeartbeatTimeout: Get<BlockNumberFor<Self>>;
	}

	/// Status, network per member
	#[pallet::storage]
	pub type Status<T: Config> = StorageMap<_, Blake2_128Concat, PeerId, MemberState, OptionQuery>;

	/// Unassigned members by network
	#[pallet::storage]
	pub type Unassigned<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, PeerId, (), OptionQuery>;

	/// Heartbeat for assigned members
	#[pallet::storage]
	pub type Heartbeat<T: Config> =
		StorageMap<_, Blake2_128Concat, PeerId, BlockNumberFor<T>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MemberJoined(Network, PeerId),
		MemberLeft(Network, PeerId),
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyMember,
		NotMember,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Join members
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn join(origin: OriginFor<T>, network: Network) -> DispatchResult {
			let member: PeerId = ensure_signed(origin)?.into();
			ensure!(Status::<T>::get(member).is_none(), Error::<T>::AlreadyMember);
			Status::<T>::insert(member, MemberState::new(network));
			Unassigned::<T>::insert(network, member, ());
			Self::assign_members();
			Self::deposit_event(Event::MemberJoined(network, member));
			Ok(())
		}

		/// Leave members
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::default())]
		pub fn leave(origin: OriginFor<T>) -> DispatchResult {
			Self::remove_member(ensure_signed(origin)?.into())
		}
	}

	impl<T: Config> Pallet<T> {
		fn assign_members() {
			for _network in Network::iter() {
				todo!()
				//Unassigned::<T>::iter_prefix(network)
				// check if more than MinShard and register_shard if so
				// TODO: how is collector chosen for new shard
				// THEN:
				// insert heartbeat upon assignment and update Status
				// callback to member_online
			}
		}
		fn remove_member(member: PeerId) -> DispatchResult {
			let MemberState { network, status } =
				Status::<T>::take(member).ok_or(Error::<T>::NotMember)?;
			if status.assigned() {
				// TODO: callback to member_offline
				Heartbeat::<T>::remove(member);
			} else {
				Unassigned::<T>::remove(network, member);
			}
			Self::deposit_event(Event::MemberLeft(network, member));
			Ok(())
		}
	}
	// MemberCreated trait
	// member_online, member_offline hooks
}
