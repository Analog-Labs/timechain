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
	use frame_system::offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, SignMessage, Signer,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{IdentifyAccount, One, Saturating};
	use sp_std::vec;
	use time_primitives::{
		AccountId, HeartbeatInfo, MemberEvents, MemberStorage, Network, PeerId, PublicKey, TxError,
		TxResult,
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

	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<Call<Self>, Public = PublicKey>
		+ frame_system::Config<AccountId = AccountId>
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type Elections: MemberEvents;
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
		) -> DispatchResult {
			let member = ensure_signed(origin)?;
			ensure!(member == public_key.clone().into_account(), Error::<T>::InvalidPublicKey);
			ensure!(MemberNetwork::<T>::get(&member).is_none(), Error::<T>::AlreadyMember);
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

		pub fn submit_register_member(
			network: Network,
			public_key: PublicKey,
			peer_id: PeerId,
		) -> TxResult {
			let signer =
				Signer::<T, T::AuthorityId>::any_account().with_filter(vec![public_key.clone()]);
			signer
				.send_signed_transaction(|_| Call::register_member {
					network,
					public_key: public_key.clone(),
					peer_id,
				})
				.ok_or(TxError::MissingSigningKey)?
				.1
				.map_err(|_| TxError::TxPoolError)
		}

		pub fn submit_heartbeat(public_key: PublicKey) -> TxResult {
			let signer = Signer::<T, T::AuthorityId>::any_account().with_filter(vec![public_key]);
			let signer_pub =
				signer.sign_message(b"temp_msg").ok_or(TxError::MissingSigningKey)?.0.public;
			let account_id = signer_pub.into_account();

			for i in 0..100 {
				let result = signer
					.send_signed_transaction(|_| Call::send_heartbeat {})
					.ok_or(TxError::MissingSigningKey)?
					.1
					.map_err(|_| TxError::TxPoolError);
				if i == 99 {
					return result;
				}

				if result.is_ok() {
					return Ok(());
				}
				log::error!("failed to send tx, retrying {}", i);
				let mut account_data = frame_system::Account::<T>::get(&account_id);
				account_data.nonce = account_data.nonce.saturating_add(One::one());
				frame_system::Account::<T>::insert(&account_id, account_data);
				continue;
			}
			Err(TxError::TxPoolError)
		}

		pub fn get_heartbeat_timeout() -> BlockNumberFor<T> {
			T::HeartbeatTimeout::get()
		}
	}

	impl<T: Config> MemberStorage for Pallet<T> {
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
