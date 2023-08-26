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
	use time_primitives::{
		AccountId, ElectionsInterface, MemberAssignment, MemberElections, Network, PeerId,
	};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		//type WeightInfo: WeightInfo;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type Elections: ElectionsInterface;
	}

	/// Get network for member
	#[pallet::storage]
	pub type MemberNetwork<T: Config> =
		StorageMap<_, Blake2_128Concat, PeerId, Network, OptionQuery>;

	/// Unassigned members per network
	#[pallet::storage]
	pub type Unassigned<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, PeerId, (), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RegisteredMember(Network, PeerId),
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyMember,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn register_member(origin: OriginFor<T>, network: Network) -> DispatchResult {
			let member: PeerId = ensure_signed(origin)?.into();
			ensure!(MemberNetwork::<T>::get(member).is_none(), Error::<T>::AlreadyMember);
			MemberNetwork::<T>::insert(member, network);
			Self::unassign_member(member, network);
			Self::deposit_event(Event::RegisteredMember(network, member));
			Ok(())
		}
	}

	impl<T: Config> MemberAssignment for Pallet<T> {
		fn assign_member(member: PeerId, network: Network) {
			Unassigned::<T>::remove(network, member);
		}
		fn unassign_member(member: PeerId, network: Network) {
			Unassigned::<T>::insert(network, member, ());
			T::Elections::unassigned_member_online(network);
		}
	}

	impl<T: Config> MemberElections for Pallet<T> {
		fn new_shard_members(n: usize, network: Network) -> Option<Vec<PeerId>> {
			let members = Unassigned::<T>::iter_prefix(network)
				.map(|(m, _)| m)
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
