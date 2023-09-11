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
	use frame_system::offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
	};
	use frame_system::pallet_prelude::*;
	use schnorr_evm::VerifyingKey;
	use sp_runtime::Saturating;
	use sp_std::vec;
	use sp_std::vec::Vec;
	use time_primitives::{
		AccountId, Commitment, ElectionsInterface, MemberEvents, MemberStatus, MemberStorage,
		Network, ProofOfKnowledge, PublicKey, ShardId, ShardStatus, ShardsInterface,
		TasksInterface, TssPublicKey, TxError, TxResult,
	};

	pub trait WeightInfo {
		fn commit() -> Weight;
		fn ready() -> Weight;
	}

	impl WeightInfo for () {
		fn commit() -> Weight {
			Weight::default()
		}

		fn ready() -> Weight {
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
	pub type ShardCommitment<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, Commitment, OptionQuery>;

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
		MemberStatus,
		OptionQuery,
	>;

	#[pallet::storage]
	pub type PastSigners<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ShardId,
		Blake2_128Concat,
		PublicKey,
		(),
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New shard was created
		ShardCreated(ShardId, Network),
		/// Shard commited
		ShardCommitted(ShardId, Commitment),
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
		UnexpectedCommit,
		InvalidCommitment,
		InvalidProofOfKnowledge,
		UnexpectedReady,
		ShardAlreadyOffline,
		OfflineShardMayNotGoOnline,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::commit())]
		pub fn commit(
			origin: OriginFor<T>,
			shard_id: ShardId,
			commitment: Commitment,
			_proof_of_knowledge: ProofOfKnowledge,
		) -> DispatchResult {
			let member = ensure_signed(origin)?;
			ensure!(
				ShardMembers::<T>::get(shard_id, &member) == Some(MemberStatus::Added),
				Error::<T>::UnexpectedCommit
			);
			let threshold = ShardThreshold::<T>::get(shard_id).unwrap_or_default();
			ensure!(commitment.len() == threshold as usize, Error::<T>::InvalidCommitment);
			for c in &commitment {
				ensure!(VerifyingKey::from_bytes(*c).is_ok(), Error::<T>::InvalidCommitment);
			}
			// TODO: verify proof of knowledge
			ShardMembers::<T>::insert(shard_id, member, MemberStatus::Committed(commitment));
			if ShardMembers::<T>::iter_prefix(shard_id).all(|(_, status)| status.is_committed()) {
				let commitment = ShardMembers::<T>::iter_prefix(shard_id)
					.filter_map(|(_, status)| status.commitment().cloned())
					.reduce(|mut group_commitment, commitment| {
						for (group_commitment, commitment) in
							group_commitment.iter_mut().zip(commitment.iter())
						{
							*group_commitment = VerifyingKey::new(
								VerifyingKey::from_bytes(*group_commitment).unwrap().to_element()
									+ VerifyingKey::from_bytes(*commitment).unwrap().to_element(),
							)
							.to_bytes()
							.unwrap();
						}
						group_commitment
					})
					.unwrap();
				ShardCommitment::<T>::insert(shard_id, commitment.clone());
				ShardState::<T>::insert(shard_id, ShardStatus::Committed);
				Self::deposit_event(Event::ShardCommitted(shard_id, commitment))
			}
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::ready())]
		pub fn ready(origin: OriginFor<T>, shard_id: ShardId) -> DispatchResult {
			let member = ensure_signed(origin)?;
			ensure!(
				matches!(
					ShardMembers::<T>::get(shard_id, &member),
					Some(MemberStatus::Committed(_))
				),
				Error::<T>::UnexpectedReady,
			);
			let network = ShardNetwork::<T>::get(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let commitment = ShardCommitment::<T>::get(shard_id).ok_or(Error::<T>::UnknownShard)?;
			ShardMembers::<T>::insert(shard_id, member, MemberStatus::Ready);
			if ShardMembers::<T>::iter_prefix(shard_id)
				.all(|(_, status)| status == MemberStatus::Ready)
			{
				<ShardState<T>>::insert(shard_id, ShardStatus::Online);
				Self::deposit_event(Event::ShardOnline(shard_id, commitment[0]));
				T::TaskScheduler::shard_online(shard_id, network);
			}
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

		pub fn get_shard_threshold(shard_id: ShardId) -> u16 {
			ShardThreshold::<T>::get(shard_id).unwrap_or_default()
		}

		pub fn get_shard_status(shard_id: ShardId) -> ShardStatus<BlockNumberFor<T>> {
			ShardState::<T>::get(shard_id).unwrap_or_default()
		}

		pub fn get_shard_commitment(shard_id: ShardId) -> Vec<TssPublicKey> {
			ShardCommitment::<T>::get(shard_id).unwrap_or_default()
		}

		pub fn submit_commitment(
			shard_id: ShardId,
			member: PublicKey,
			commitment: Commitment,
			proof_of_knowledge: ProofOfKnowledge,
		) -> TxResult {
			let signer = Signer::<T, T::AuthorityId>::any_account().with_filter(vec![member]);
			signer
				.send_signed_transaction(|_| Call::commit {
					shard_id,
					commitment: commitment.clone(),
					proof_of_knowledge,
				})
				.ok_or(TxError::MissingSigningKey)?
				.1
				.map_err(|_| TxError::TxPoolError)
		}

		pub fn submit_online(shard_id: ShardId, member: PublicKey) -> TxResult {
			let signer = Signer::<T, T::AuthorityId>::any_account().with_filter(vec![member]);
			signer
				.send_signed_transaction(|_| Call::ready { shard_id })
				.ok_or(TxError::MissingSigningKey)?
				.1
				.map_err(|_| TxError::TxPoolError)
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
				ShardMembers::<T>::insert(shard_id, member, MemberStatus::Added);
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
			let mut signer_index = rng.usize(..members.len());
			let mut signer = T::Members::member_public_key(&members[signer_index])
				.expect("All signers should be registered members");
			if members.len() == 1 {
				// only one possible signer for shard size 1
				return signer;
			}
			let mut retry_count = 0;
			while PastSigners::<T>::get(shard_id, &signer).is_some() && retry_count <= members.len()
			{
				signer_index = if signer_index == members.len() - 1 { 0 } else { signer_index + 1 };
				signer = T::Members::member_public_key(&members[signer_index])
					.expect("All signers should be registered members");
				retry_count += 1;
			}
			PastSigners::<T>::insert(shard_id, &signer, ());
			signer
		}

		fn tss_public_key(shard_id: ShardId) -> Option<TssPublicKey> {
			ShardCommitment::<T>::get(shard_id).map(|commitment| commitment[0])
		}
	}
}
