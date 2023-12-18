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
	use frame_support::traits::{Currency, ReservableCurrency};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{IdentifyAccount, Saturating};
	use sp_std::vec;
	use time_primitives::{
		AccountId, HeartbeatInfo, MemberEvents, MemberStorage, Network, PeerId, PublicKey,
	};

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

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Elections: MemberEvents;
		/// The currency type
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Minimum stake to register member
		#[pallet::constant]
		type MinStake: Get<BalanceOf<Self>>;
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

	/// Get PublicKey for member
	#[pallet::storage]
	pub type MemberPublicKey<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, PublicKey, OptionQuery>;

	/// Indicate if member is online or offline
	#[pallet::storage]
	pub type Heartbeat<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, HeartbeatInfo<BlockNumberFor<T>>, OptionQuery>;

	/// Get stake for member
	#[pallet::storage]
	pub type MemberStake<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, BalanceOf<T>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RegisteredMember(AccountId, Network, PeerId),
		HeartbeatReceived(AccountId),
		MemberOnline(AccountId),
		MemberOffline(AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidPublicKey,
		AlreadyMember,
		NotMember,
		BondBelowMinStake,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut writes = 0;
			Heartbeat::<T>::iter().for_each(|(member, heart)| {
				if heart.is_online && n.saturating_sub(heart.block) >= T::HeartbeatTimeout::get() {
					Heartbeat::<T>::insert(&member, heart.set_offline());
					Self::member_offline(&member);
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
			public_key: PublicKey,
			peer_id: PeerId,
			bond: BalanceOf<T>,
		) -> DispatchResult {
			let member = ensure_signed(origin)?;
			ensure!(member == public_key.clone().into_account(), Error::<T>::InvalidPublicKey);
			ensure!(MemberNetwork::<T>::get(&member).is_none(), Error::<T>::AlreadyMember);
			ensure!(bond >= T::MinStake::get(), Error::<T>::BondBelowMinStake);
			T::Currency::reserve(&member, bond)?;
			MemberStake::<T>::insert(&member, bond);
			MemberNetwork::<T>::insert(&member, network);
			MemberPublicKey::<T>::insert(&member, public_key);
			MemberPeerId::<T>::insert(&member, peer_id);
			Heartbeat::<T>::insert(
				&member,
				HeartbeatInfo::new(frame_system::Pallet::<T>::block_number()),
			);
			Self::deposit_event(Event::RegisteredMember(member.clone(), network, peer_id));
			Self::member_online(&member);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::send_heartbeat())]
		pub fn send_heartbeat(origin: OriginFor<T>) -> DispatchResult {
			let member = ensure_signed(origin)?;
			let heart = Heartbeat::<T>::get(&member).ok_or(Error::<T>::NotMember)?;
			Heartbeat::<T>::insert(
				&member,
				HeartbeatInfo::new(frame_system::Pallet::<T>::block_number()),
			);
			Self::deposit_event(Event::HeartbeatReceived(member.clone()));
			if !heart.is_online {
				Self::member_online(&member);
			}
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn member_online(member: &AccountId) {
			Self::deposit_event(Event::MemberOnline(member.clone()));
			if let Some(network) = MemberNetwork::<T>::get(member) {
				T::Elections::member_online(member, network);
			}
		}

		fn member_offline(member: &AccountId) {
			Self::deposit_event(Event::MemberOffline(member.clone()));
			if let Some(network) = MemberNetwork::<T>::get(member) {
				T::Elections::member_offline(member, network);
			}
		}

		pub fn get_heartbeat_timeout() -> BlockNumberFor<T> {
			T::HeartbeatTimeout::get()
		}
	}

	impl<T: Config> MemberStorage for Pallet<T> {
		type Balance = BalanceOf<T>;
		fn member_stake(account: &AccountId) -> BalanceOf<T> {
			MemberStake::<T>::get(account).unwrap_or_default()
		}

		fn member_peer_id(account: &AccountId) -> Option<PeerId> {
			MemberPeerId::<T>::get(account)
		}

		fn member_public_key(account: &AccountId) -> Option<PublicKey> {
			MemberPublicKey::<T>::get(account)
		}

		fn is_member_online(account: &AccountId) -> bool {
			let Some(heart) = Heartbeat::<T>::get(account) else { return false };
			frame_system::Pallet::<T>::block_number().saturating_sub(heart.block)
				< T::HeartbeatTimeout::get()
		}
	}
}
