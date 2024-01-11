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
	use frame_support::{
		pallet_prelude::{ValueQuery, *},
		traits::{Currency, ExistenceRequirement},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use schnorr_evm::VerifyingKey;
	use sp_runtime::{traits::AccountIdConversion, Perbill, Saturating};
	use sp_std::vec;
	use sp_std::vec::Vec;
	use time_primitives::{
		AccountId, Commitment, ElectionsInterface, MemberEvents, MemberStatus, MemberStorage,
		Network, ProofOfKnowledge, PublicKey, ShardId, ShardStatus, ShardsInterface,
		TasksInterface, TssPublicKey,
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

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = sp_runtime::AccountId32> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Elections: ElectionsInterface;
		type Members: MemberStorage;
		type TaskScheduler: TasksInterface;
		type Currency: Currency<Self::AccountId>;
		#[pallet::constant]
		type DkgTimeout: Get<BlockNumberFor<Self>>;
		/// `PalletId` for the shard pallet. An appropriate value could be
		/// `PalletId(*b"py/shard")`
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type MinShardBalanceForAutoPayout: Get<BalanceOf<Self>>;
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
		/// Shard paid out rewards to all of its members
		ShardPaidRewards(ShardId, BalanceOf<T>),
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

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::ready())]//TODO: update weights
		pub fn pay_rewards(origin: OriginFor<T>, shard_id: ShardId) -> DispatchResult {
			ensure_signed(origin)?;
			Self::payout_shard_members_rewards(shard_id);
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
		// Payout shard members whenever there is block space available
		// and shard balance is above `MinShardBalanceForAutoPayout`
		fn on_idle(_n: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let mut weight_used = Weight::default();
			for (shard_id, _) in ShardState::<T>::iter() {
				// if shard has sufficient balance, auto payout to members
				if Self::shard_balance(shard_id) >= T::MinShardBalanceForAutoPayout::get() {
					let weight_consumed = Self::payout_shard_members_rewards(shard_id);
					weight_used.saturating_add(weight_consumed);
				}
				// each ShardState is 1 read
				weight_used = weight_used.saturating_add(T::DbWeight::get().read.into());
				if weight_used.any_lt(remaining_weight) {
					break;
				}
			}
			weight_used
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

		pub fn get_shard_members(shard_id: ShardId) -> Vec<(AccountId, MemberStatus)> {
			ShardMembers::<T>::iter_prefix(shard_id)
				.map(|(time_id, status)| (time_id, status))
				.collect()
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

		/// Return weight consumed
		fn payout_shard_members_rewards(shard_id: ShardId) -> Weight {
			let shard_account = Self::shard_account(shard_id);
			let members_len = ShardMembers::<T>::iter_prefix(shard_id).collect::<Vec<_>>().len();
			let member_share = Perbill::from_rational(1u32, members_len as u32);
			let member_payout = member_share.mul_floor(T::Currency::free_balance(&shard_account));
			for (member, _) in ShardMembers::<T>::iter_prefix(shard_id) {
				T::Currency::transfer(
					&shard_account,
					&member,
					member_payout,
					ExistenceRequirement::KeepAlive,
				).expect("member_share * member_payout < T::Currency::free_balance(&shard_account because of mul_floor QED");
			}
			Self::deposit_event(Event::ShardPaidRewards(shard_id, member_payout));
			// READs: 1 shard_account read + shard_members.len() members read
			// WRITEs: shard_members.len() transfers written + 1 event emitted
			// so read_count == write_count
			let read_and_write_count = members_len
				.saturating_plus_one()
				.try_into()
				.expect("members_len + 1 usize will fit in u64");
			T::DbWeight::get().reads(read_and_write_count)
				+ T::DbWeight::get().writes(read_and_write_count)
		}
	}

	impl<T: Config> MemberEvents for Pallet<T> {
		fn member_online(id: &AccountId, network: Network) {
			let Some(shard_id) = MemberShard::<T>::get(id) else { return };
			let Some(old_status) = ShardState::<T>::get(shard_id) else { return };
			let new_status = old_status.online_member();
			ShardState::<T>::insert(shard_id, new_status);
			if matches!(new_status, ShardStatus::Online)
				&& !matches!(old_status, ShardStatus::Online)
			{
				T::TaskScheduler::shard_online(shard_id, network);
			}
		}

		fn member_offline(id: &AccountId, _: Network) {
			let Some(shard_id) = MemberShard::<T>::get(id) else { return };
			let Some(old_status) = ShardState::<T>::get(shard_id) else { return };
			let Some(shard_threshold) = ShardThreshold::<T>::get(shard_id) else { return };
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
		type Balance = BalanceOf<T>;
		fn is_shard_online(shard_id: ShardId) -> bool {
			matches!(ShardState::<T>::get(shard_id), Some(ShardStatus::Online))
		}

		fn is_shard_member(member: &AccountId) -> bool {
			MemberShard::<T>::get(member).is_some()
		}

		fn shard_network(shard_id: ShardId) -> Option<Network> {
			ShardNetwork::<T>::get(shard_id)
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
			let mut signer = T::Members::member_public_key(&members[signer_index].0)
				.expect("All signers should be registered members");
			if members.len() == 1 {
				// only one possible signer for shard size 1
				return signer;
			}
			if PastSigners::<T>::iter_prefix(shard_id).count() < members.len() {
				while PastSigners::<T>::get(shard_id, &signer).is_some() {
					signer_index =
						if signer_index == members.len() - 1 { 0 } else { signer_index + 1 };
					signer = T::Members::member_public_key(&members[signer_index].0)
						.expect("All signers should be registered members");
				}
			}
			PastSigners::<T>::insert(shard_id, &signer, ());
			signer
		}

		fn tss_public_key(shard_id: ShardId) -> Option<TssPublicKey> {
			ShardCommitment::<T>::get(shard_id).map(|commitment| commitment[0])
		}

		fn shard_account(shard_id: ShardId) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(shard_id)
		}

		fn shard_balance(shard_id: ShardId) -> BalanceOf<T> {
			T::Currency::free_balance(&Self::shard_account(shard_id))
		}
	}
}
