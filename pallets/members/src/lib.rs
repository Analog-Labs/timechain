#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Saturating;
	use time_primitives::{AccountId, HeartbeatInfo, MemberEvents, MemberStorage, Network, PeerId};

	pub trait WeightInfo {
		fn register_member() -> Weight;
		fn send_heartbeat() -> Weight;
	}

	impl WeightInfo for () {
		fn register_member() -> Weight {
			Weight::default()
		}
		fn send_heartbeat() -> Weight {
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
		type Shards: MemberEvents;
		#[pallet::constant]
		type HeartbeatTimeout: Get<BlockNumberFor<Self>>;
	}

	/// Get network for member
	#[pallet::storage]
	pub type MemberNetwork<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, Network, OptionQuery>;

	/// Get PeerId for member
	#[pallet::storage]
	pub type MemberPeerId<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, PeerId, OptionQuery>;

	/// Indicate if member is online or offline
	#[pallet::storage]
	pub type Heartbeat<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, HeartbeatInfo<BlockNumberFor<T>>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RegisteredMember(AccountId, Network, PeerId),
		HeartbeatReceived(AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyMember,
		NotMember,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut writes = 0;
			Heartbeat::<T>::iter().for_each(|(account, heart)| {
				if heart.is_online && n.saturating_sub(heart.block) >= T::HeartbeatTimeout::get() {
					T::Shards::member_offline(&account);
					Heartbeat::<T>::insert(&account, heart.set_offline());
					writes += 1;
				}
			});
			T::DbWeight::get().writes(writes)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register_member())]
		pub fn register_member(
			origin: OriginFor<T>,
			network: Network,
			id: PeerId,
		) -> DispatchResult {
			let member = ensure_signed(origin)?;
			ensure!(MemberNetwork::<T>::get(&member).is_none(), Error::<T>::AlreadyMember);
			MemberNetwork::<T>::insert(&member, network);
			MemberPeerId::<T>::insert(&member, id);
			Heartbeat::<T>::insert(
				&member,
				HeartbeatInfo::new(frame_system::Pallet::<T>::block_number()),
			);
			Self::deposit_event(Event::RegisteredMember(member, network, id));
			Ok(())
		}
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::send_heartbeat())]
		pub fn send_heartbeat(origin: OriginFor<T>) -> DispatchResult {
			let member = ensure_signed(origin)?;
			let heart = Heartbeat::<T>::get(&member).ok_or(Error::<T>::NotMember)?;
			if !heart.is_online {
				T::Shards::member_online(&member);
			}
			Heartbeat::<T>::insert(
				&member,
				HeartbeatInfo::new(frame_system::Pallet::<T>::block_number()),
			);
			Self::deposit_event(Event::HeartbeatReceived(member));
			Ok(())
		}
	}

	impl<T: Config> MemberStorage for Pallet<T> {
		fn member_peer_id(account: AccountId) -> Option<PeerId> {
			MemberPeerId::<T>::get(account)
		}
	}
}
