#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::{ValueQuery, *};
	use frame_system::offchain::{AppCrypto, CreateSignedTransaction};
	use frame_system::pallet_prelude::*;
	use sp_runtime::Saturating;
	use sp_std::vec::Vec;
	use time_primitives::{
		AccountId, ElectionsInterface, MemberEvents, MemberStorage, Network, PublicKey, ShardId,
		ShardStatus, ShardsInterface, TasksInterface, TssPublicKey,
	};

	pub trait WeightInfo {
		fn register_shard() -> Weight;
		fn submit_tss_public_key() -> Weight;
	}

	impl WeightInfo for () {
		fn register_shard() -> Weight {
			Weight::default()
		}

		fn submit_tss_public_key() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<Call<Self>, Public = PublicKey>
		+ frame_system::Config<AccountId = sp_runtime::AccountId32>
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type Elections: ElectionsInterface;
		type Members: MemberStorage;
		type TaskScheduler: TasksInterface;
		#[pallet::constant]
		type DkgTimeout: Get<BlockNumberFor<Self>>;
	}

	#[pallet::storage]
	/// Counter for creating unique shard_ids during on-chain creation
	pub type ShardIdCounter<T: Config> = StorageValue<_, ShardId, ValueQuery>;

	/// Network for which shards can be assigned tasks
	#[pallet::storage]
	pub type ShardNetwork<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, Network, OptionQuery>;

	/// Status for shard
	#[pallet::storage]
	pub type ShardState<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, ShardStatus<BlockNumberFor<T>>, OptionQuery>;

	/// Threshold for shard
	#[pallet::storage]
	pub type ShardThreshold<T: Config> = StorageMap<_, Blake2_128Concat, ShardId, u16, OptionQuery>;

	#[pallet::storage]
	pub type ShardPublicKey<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, TssPublicKey, OptionQuery>;

	#[pallet::storage]
	pub type MemberShard<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, ShardId, OptionQuery>;

	#[pallet::storage]
	pub type ShardMembers<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ShardId,
		Blake2_128Concat,
		AccountId,
		(),
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New shard was created
		ShardCreated(ShardId, Network),
		/// Shard DKG timed out
		ShardKeyGenTimedOut(ShardId),
		/// Shard completed dkg and submitted public key to runtime
		ShardOnline(ShardId, TssPublicKey),
		/// Shard went offline
		ShardOffline(ShardId),
	}

	#[pallet::error]
	pub enum Error<T> {
		UnknownShard,
		PublicKeyAlreadyRegistered,
		ShardAlreadyOffline,
		OfflineShardMayNotGoOnline,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::submit_tss_public_key())]
		pub fn submit_tss_public_key(
			origin: OriginFor<T>,
			shard_id: ShardId,
			public_key: TssPublicKey,
		) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(
				!matches!(ShardState::<T>::get(shard_id), Some(ShardStatus::Offline)),
				Error::<T>::OfflineShardMayNotGoOnline
			);
			ensure!(
				ShardPublicKey::<T>::get(shard_id).is_none(),
				Error::<T>::PublicKeyAlreadyRegistered
			);
			let network = ShardNetwork::<T>::get(shard_id).ok_or(Error::<T>::UnknownShard)?;
			<ShardPublicKey<T>>::insert(shard_id, public_key);
			<ShardState<T>>::insert(shard_id, ShardStatus::Online);
			Self::deposit_event(Event::ShardOnline(shard_id, public_key));
			T::TaskScheduler::shard_online(shard_id, network);
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut writes = 0;
			ShardState::<T>::iter().for_each(|(shard_id, status)| {
				if let Some(created_block) = status.when_created() {
					if n.saturating_sub(created_block) >= T::DkgTimeout::get() {
						Self::remove_shard_offline(shard_id);
						Self::deposit_event(Event::ShardKeyGenTimedOut(shard_id));
						writes += 5;
					}
				}
			});
			T::DbWeight::get().writes(writes)
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if !matches!(source, TransactionSource::Local | TransactionSource::InBlock) {
				return InvalidTransaction::Call.into();
			}

			let is_valid = match call {
				Call::submit_tss_public_key { shard_id, .. } => {
					log::info!("got unsigned tx for tss pub key {:?}", shard_id);
					ShardPublicKey::<T>::get(shard_id).is_none()
				},
				_ => false,
			};

			if is_valid {
				ValidTransaction::with_tag_prefix("shards-pallet")
					.priority(TransactionPriority::max_value())
					.longevity(10)
					.propagate(true)
					.build()
			} else {
				return InvalidTransaction::Call.into();
			}
		}

		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Remove shard state for shard that just went offline
		/// Set shard status to offline and keep shard public key if already submitted
		fn remove_shard_offline(shard_id: ShardId) {
			ShardState::<T>::insert(shard_id, ShardStatus::Offline);
			ShardThreshold::<T>::remove(shard_id);
			let Some(network) = ShardNetwork::<T>::take(shard_id) else { return };
			T::TaskScheduler::shard_offline(shard_id, network);
			let members = ShardMembers::<T>::drain_prefix(shard_id)
				.map(|(m, _)| {
					MemberShard::<T>::remove(&m);
					m
				})
				.collect::<Vec<_>>();
			T::Elections::shard_offline(network, members);
		}

		pub fn get_shard_threshold(shard_id: ShardId) -> u16 {
			ShardThreshold::<T>::get(shard_id).unwrap_or_default()
		}

		pub fn get_shards(account: &AccountId) -> Vec<ShardId> {
			ShardMembers::<T>::iter()
				.filter_map(
					|(shard_id, member, _)| {
						if member == *account {
							Some(shard_id)
						} else {
							None
						}
					},
				)
				.collect()
		}

		pub fn get_shard_members(shard_id: ShardId) -> Vec<AccountId> {
			ShardMembers::<T>::iter_prefix(shard_id).map(|(time_id, _)| time_id).collect()
		}

		pub fn submit_tss_pub_key(shard_id: ShardId, public_key: TssPublicKey) {
			use frame_system::offchain::SubmitTransaction;
			let call = Call::submit_tss_public_key { shard_id, public_key };
			let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
			log::info!("submitted pubkey {:?}", res);
		}
	}

	impl<T: Config> MemberEvents for Pallet<T> {
		fn member_online(id: &AccountId, network: Network) {
			let Some(shard_id) = MemberShard::<T>::get(id) else  { return };
			let Some(old_status) = ShardState::<T>::get(shard_id) else  { return };
			let new_status = old_status.online_member();
			ShardState::<T>::insert(shard_id, new_status);
			if matches!(new_status, ShardStatus::Online)
				&& !matches!(old_status, ShardStatus::Online)
			{
				T::TaskScheduler::shard_online(shard_id, network);
			}
		}

		fn member_offline(id: &AccountId, _: Network) {
			let Some(shard_id) = MemberShard::<T>::get(id) else  { return };
			let Some(old_status) = ShardState::<T>::get(shard_id) else  { return };
			let Some(shard_threshold) = ShardThreshold::<T>::get(shard_id) else  { return };
			let total_members = Self::get_shard_members(shard_id).len();
			let max_members_offline = total_members.saturating_sub(shard_threshold.into());
			let Ok(max_members_offline) = max_members_offline.try_into() else { return };
			let new_status = old_status.offline_member(max_members_offline);
			if matches!(new_status, ShardStatus::Offline)
				&& !matches!(old_status, ShardStatus::Offline)
			{
				Self::remove_shard_offline(shard_id);
				Self::deposit_event(Event::ShardOffline(shard_id));
			} else if !matches!(new_status, ShardStatus::Offline) {
				ShardState::<T>::insert(shard_id, new_status);
			}
		}
	}

	impl<T: Config> ShardsInterface for Pallet<T> {
		fn is_shard_online(shard_id: ShardId) -> bool {
			matches!(ShardState::<T>::get(shard_id), Some(ShardStatus::Online))
		}

		fn is_shard_member(member: &AccountId) -> bool {
			MemberShard::<T>::get(member).is_some()
		}

		fn create_shard(network: Network, members: Vec<AccountId>, threshold: u16) {
			let shard_id = <ShardIdCounter<T>>::get();
			<ShardIdCounter<T>>::put(shard_id + 1);
			<ShardNetwork<T>>::insert(shard_id, network);
			<ShardState<T>>::insert(
				shard_id,
				ShardStatus::Created(frame_system::Pallet::<T>::block_number()),
			);
			<ShardThreshold<T>>::insert(shard_id, threshold);
			for member in &members {
				ShardMembers::<T>::insert(shard_id, member, ());
				MemberShard::<T>::insert(member, shard_id);
			}
			Self::deposit_event(Event::ShardCreated(shard_id, network));
		}

		fn random_signer(shard_id: ShardId) -> PublicKey {
			let seed = u64::from_ne_bytes(
				frame_system::Pallet::<T>::parent_hash().encode().as_slice()[0..8]
					.try_into()
					.expect("Block hash should convert into [u8; 8]"),
			);
			let mut rng = fastrand::Rng::with_seed(seed);
			let members = Self::get_shard_members(shard_id);
			T::Members::member_public_key(&members[rng.usize(..members.len())])
				.expect("All signers should be registered members")
		}
	}
}
