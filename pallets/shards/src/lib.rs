#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]
//! # Timechain Shards Pallet
//!
//! The Shards pallet manages the lifecycle of shards in a decentralized network. It handles the
//! creation, commitment, readiness, and offline status of shards, along with managing shard members
//! and their state. The pallet ensures that the commitments of all members are valid and that they
//! are ready before transitioning a shard to an online state. It also provides mechanisms for
//! forcefully taking shards offline when needed. The main callable functions (commit, ready, and
//! force_shard_offline) enable members and administrators to interact with the shards, while hooks
//! like on_initialize ensure timely state updates and cleanup. Events are emitted to signal
//! important state changes and actions taken on shards.
//!
//! ## Call Functions
//!
//! This graph represents the workflow of the `commit`, `ready`, and `force_shard_offline` call
//! functions within the Shards pallet.
//!
//! ### Commit Flow
//!
//! The commit process begins with verifying that the request is from an authenticated user. It then
//! checks the current status of the member to ensure they are eligible to commit. If the status is
//! not as expected, the process returns an `UnexpectedCommit` error. Once the status is validated,
//! the required commitment threshold is retrieved, and the length of the commitment is checked for
//! appropriateness. The commitment is validated, and any invalid commitment results in an
//! `InvalidCommitment` error. A valid commitment is stored, followed by a check to see if all
//! necessary commitments have been received. Once all commitments are collected, they are aggregated
//! into a group commitment, which is then stored. The shard state is updated based on these
//! commitments, and the process concludes with the logging of a `ShardCommitted` event.
//!
//! ### Force Shard Offline Flow
//!
//! The force shard offline process starts with ensuring the request is from a root user. Upon
//! confirmation, the system calls the `remove_shard_offline` function to begin the shard removal
//! process. This involves removing the shard state, retrieving the network details, and scheduling
//! the `shard_offline` task. The process also includes draining and removing shard members,
//! removing members from the `MemberShard`, and concludes with logging the `ShardOffline` event.
//!
//! ### Ready Flow
//!
//! The ready process begins with ensuring the request is from an authenticated user. It checks the
//! current status of the member to confirm they are in the correct state to be marked as ready. If
//! the status is not appropriate, an `UnexpectedReady` error is returned. Once the status is
//! validated, the system retrieves the network and commitment of the member. It is then marked as
//! ready, and a check is performed to see if all members are ready. If all members are ready, the
//! shard state is updated to `Online`, and the `shard_online` task is scheduled. The process ends
//! with the logging of a `ShardOnline` event.
//!  
#![doc = simple_mermaid::mermaid!("../docs/shard_callfunctions.mmd")]
//!
//! ## **on_initialize Hook**
//!
//! This graph illustrates the workflow of the `on_initialize` function within the Shards pallet.
//! The `on_initialize` function is triggered at the beginning of each block and iterates over the
//! `DkgTimeout` entries to identify any shards that have timed out. For each entry, the function
//! checks if the timeout condition is met. If the condition is met, the function handles the timeout
//! by either removing the `DkgTimeout` entry or marking the shard as offline. In the case where the
//! shard is neither in a Created nor Committed state, the function removes the timeout entry.
//! If the shard is in a Created or Committed state, it proceeds to handle the shard going offline.
//! This involves removing the state and thresholds entrie of a shard, attempting to retrieve the
//! associated network, and marking the shard as offline in the task scheduler. Additionally, the
//! function drains the shard members, removes their entries, handles the shard going offline in the
//! elections module, and finally emits the `ShardOffline` event.
//!
#![doc = simple_mermaid::mermaid!("../docs/shard_hook.mmd")]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use polkadot_sdk::{frame_support, frame_system, pallet_balances, sp_runtime, sp_std};

	use frame_support::pallet_prelude::{EnsureOrigin, ValueQuery, *};
	use frame_system::pallet_prelude::*;

	use schnorr_evm::VerifyingKey;
	use sp_runtime::Saturating;
	use sp_std::collections::btree_map::BTreeMap;
	use sp_std::vec;
	use sp_std::vec::Vec;

	use time_primitives::{
		AccountId, Balance, Commitment, ElectionsInterface, MemberStatus, MembersInterface,
		NetworkId, ProofOfKnowledge, PublicKey, ShardId, ShardStatus, ShardsInterface,
		TasksInterface, TssPublicKey,
	};

	/// Trait to define the weights for various extrinsics in the pallet.
	pub trait WeightInfo {
		fn commit() -> Weight;
		fn ready() -> Weight;
		fn force_shard_offline() -> Weight;
		fn timeout_dkgs(b: u32) -> Weight;
	}

	impl WeightInfo for () {
		fn commit() -> Weight {
			Weight::default()
		}

		fn ready() -> Weight {
			Weight::default()
		}

		fn force_shard_offline() -> Weight {
			Weight::default()
		}

		fn timeout_dkgs(_: u32) -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<AccountId = AccountId>
		+ pallet_balances::Config<Balance = Balance>
	{
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type WeightInfo: WeightInfo;
		type Elections: ElectionsInterface;
		type Members: MembersInterface;
		type Tasks: TasksInterface;
		#[pallet::constant]
		type DkgTimeout: Get<BlockNumberFor<Self>>;
	}

	#[pallet::storage]
	/// Counter for creating unique shard_ids during on-chain creation
	pub type ShardIdCounter<T: Config> = StorageValue<_, ShardId, ValueQuery>;

	/// Maps `ShardId` to `NetworkId` indicating the network for which shards can be assigned tasks.
	#[pallet::storage]
	pub type ShardNetwork<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, NetworkId, OptionQuery>;

	/// Maps `ShardId` to `ShardStatus` indicating the status of each shard.
	#[pallet::storage]
	pub type ShardState<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, ShardStatus, OptionQuery>;

	/// Maps `BlockNumber` to the number of shards scheduled to timeout
	#[pallet::storage]
	pub type DkgTimeoutCounter<T: Config> =
		StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, u32, ValueQuery>;

	/// Tracks `BlockNumber` at which the shard with `ShardId` will DKG timeout.
	#[pallet::storage]
	pub type DkgTimeout<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		Blake2_128Concat,
		ShardId,
		(),
		OptionQuery,
	>;

	/// Maps `ShardId` to `u16` indicating the threshold for each shard.
	#[pallet::storage]
	pub type ShardThreshold<T: Config> = StorageMap<_, Blake2_128Concat, ShardId, u16, OptionQuery>;

	/// Maps `ShardId` to `Commitment` indicating the commitment of each shard.
	#[pallet::storage]
	pub type ShardCommitment<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, Commitment, OptionQuery>;

	/// Maps `AccountId` to `ShardId` indicating the shard a member is part of.
	#[pallet::storage]
	pub type MemberShard<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, ShardId, OptionQuery>;

	/// Maps `ShardId` to `u32` indicating the signer index for each shard.
	#[pallet::storage]
	pub type SignerIndex<T: Config> = StorageMap<_, Blake2_128Concat, ShardId, u32, ValueQuery>;

	/// Double map storing the `MemberStatus` of each `AccountId` in a specific ShardId.
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

	/// Maps `ShardId` to `u16` indicating the number of online members in each shard.
	#[pallet::storage]
	pub type ShardMembersOnline<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, u16, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New shard was created
		ShardCreated(ShardId, NetworkId),
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
		/// Indicates that the specified shard network does not exist.
		UnknownShardNetwork,
		/// Indicates that the specified shard commitment does not exist.
		UnknownShardCommitment,
		/// Indicates that an unexpected commitment was provided for the shard.
		UnexpectedCommit,
		/// Indicates that a peer id cannot be found for the member.
		MemberPeerIdNotFound,
		/// Commitment length not equal to threshold.
		CommitmentLenNotEqualToThreshold,
		/// Verify Key in Commitment was invalid.
		InvalidVerifyingKeyInCommitment,
		/// Indicates that an invalid commitment was provided.
		InvalidCommitment,
		/// Indicates that an invalid proof of knowledge was provided.
		InvalidProofOfKnowledge,
		/// Indicates that an unexpected ready state occurred.
		UnexpectedReady,
		/// Indicates that the maximum number of shards were created this block.
		MaxShardsCreatedThisBlock,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Allows a member to submit a commitment to a shard.
		/// # Flow
		///   1. Ensure the origin is a signed transaction and the sender is a member of the shard.
		///   2. Validate the commitment length against the shard threshold.
		///   3. Validate each commitment element.
		///   4. Verify the proof of knowledge using the peer ID of the member.
		///   5. Update the status of the member to `Committed` and store the commitment.
		///   6. If all members have committed, update the state of the shards to `Committed` and store the group commitment.
		///   7. Emit the [`Event::ShardCommitted`] event.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::commit())]
		pub fn commit(
			origin: OriginFor<T>,
			shard_id: ShardId,
			commitment: Commitment,
			proof_of_knowledge: ProofOfKnowledge,
		) -> DispatchResult {
			let member = ensure_signed(origin)?;
			Self::execute_commit(member, shard_id, commitment, proof_of_knowledge)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::commit())]
		pub fn sudo_commit(
			origin: OriginFor<T>,
			member: AccountId,
			shard_id: ShardId,
			commitment: Commitment,
			proof_of_knowledge: ProofOfKnowledge,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::execute_commit(member, shard_id, commitment, proof_of_knowledge)
		}

		/// Marks a shard as ready when a member indicates readiness after commitment.
		///
		/// # Flow
		///   1. Ensure the origin is a signed transaction and the sender has committed.
		///   2. Retrieve the network and commitment of the shard.
		///   3. Update the status of the shard to `Ready`.
		///   4. If all members are ready, update the state of the shard to `Online` and emit the [`Event::ShardOnline`] event.
		///   5. Notify the task scheduler that the shard is online.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::ready())]
		pub fn ready(origin: OriginFor<T>, shard_id: ShardId) -> DispatchResult {
			let member = ensure_signed(origin)?;
			Self::execute_ready(member, shard_id)
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::ready())]
		pub fn sudo_ready(
			origin: OriginFor<T>,
			member: AccountId,
			shard_id: ShardId,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::execute_ready(member, shard_id)
		}
		/// Forces a shard to go offline, used primarily by the root.
		/// # Flow
		///   1. Ensure the origin is the root.
		///   2. Call the internal `remove_shard_offline` function to handle the shard offline process.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::force_shard_offline())]
		pub fn force_shard_offline(origin: OriginFor<T>, shard_id: ShardId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Self::remove_shard_offline(shard_id);
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			log::info!("on_initialize begin");
			let weight_consumed = Self::timeout_dkgs(n);
			log::info!("on_initialize end");
			weight_consumed
		}
	}

	impl<T: Config> Pallet<T> {
		fn execute_commit(
			member: AccountId,
			shard_id: ShardId,
			commitment: Commitment,
			proof_of_knowledge: ProofOfKnowledge,
		) -> DispatchResult {
			ensure!(
				ShardMembers::<T>::get(shard_id, &member) == Some(MemberStatus::Added),
				Error::<T>::UnexpectedCommit
			);
			let threshold = ShardThreshold::<T>::get(shard_id).unwrap_or_default();
			ensure!(
				commitment.0.len() == threshold as usize,
				Error::<T>::CommitmentLenNotEqualToThreshold
			);
			for c in &commitment.0 {
				ensure!(
					VerifyingKey::from_bytes(*c).is_ok(),
					Error::<T>::InvalidVerifyingKeyInCommitment
				);
			}
			let peer_id =
				T::Members::member_peer_id(&member).ok_or(Error::<T>::MemberPeerIdNotFound)?;
			schnorr_evm::proof_of_knowledge::verify_proof_of_knowledge(
				&peer_id,
				&commitment.0,
				proof_of_knowledge,
			)
			.map_err(|_| Error::<T>::InvalidProofOfKnowledge)?;
			ShardMembers::<T>::insert(shard_id, member, MemberStatus::Committed(commitment));
			if ShardMembers::<T>::iter_prefix(shard_id).all(|(_, status)| status.is_committed()) {
				let commitment = ShardMembers::<T>::iter_prefix(shard_id)
					.filter_map(|(_, status)| status.commitment().cloned())
					.reduce(|mut group_commitment, commitment| {
						for (group_commitment, commitment) in
							group_commitment.0.iter_mut().zip(commitment.0.iter())
						{
							*group_commitment = VerifyingKey::new(
								VerifyingKey::from_bytes(*group_commitment)
									.expect("GroupCommitment output is invalid")
									.to_element() + VerifyingKey::from_bytes(*commitment)
									.expect("Commitment is invalid")
									.to_element(),
							)
							.to_bytes()
							.expect("Group commitment construction failed");
						}
						group_commitment
					})
					.ok_or(Error::<T>::InvalidCommitment)?;
				ShardCommitment::<T>::insert(shard_id, commitment.clone());
				ShardState::<T>::insert(shard_id, ShardStatus::Committed);
				Self::deposit_event(Event::ShardCommitted(shard_id, commitment))
			}
			Ok(())
		}
		fn execute_ready(member: AccountId, shard_id: ShardId) -> DispatchResult {
			ensure!(
				matches!(
					ShardMembers::<T>::get(shard_id, &member),
					Some(MemberStatus::Committed(_))
				),
				Error::<T>::UnexpectedReady,
			);
			let network =
				ShardNetwork::<T>::get(shard_id).ok_or(Error::<T>::UnknownShardNetwork)?;
			let commitment =
				ShardCommitment::<T>::get(shard_id).ok_or(Error::<T>::UnknownShardCommitment)?;
			ShardMembers::<T>::insert(shard_id, member, MemberStatus::Ready);
			if ShardMembers::<T>::iter_prefix(shard_id)
				.all(|(_, status)| status == MemberStatus::Ready)
			{
				<ShardState<T>>::insert(shard_id, ShardStatus::Online);
				Self::deposit_event(Event::ShardOnline(shard_id, commitment.0[0]));
				T::Tasks::shard_online(shard_id, network);
			}
			Ok(())
		}
		/// Handles the internal logic for removing a shard and setting its state to offline.
		/// Set shard status to offline and keep shard public key if already submitted
		/// # Flow
		///   1. Update the state of the shard to `Offline`.
		///   2. Remove the threshold of the shard.
		///   3. Notify the task scheduler and elections module that the shard is offline.
		///   4. Drain the members of the shard and remove their corresponding entries.
		///   5. Emit the [`Event::ShardOffline`] event.
		fn remove_shard_offline(shard_id: ShardId) {
			ShardState::<T>::insert(shard_id, ShardStatus::Offline);
			ShardThreshold::<T>::remove(shard_id);
			let Some(network) = ShardNetwork::<T>::take(shard_id) else { return };
			T::Tasks::shard_offline(shard_id, network);
			let members = ShardMembers::<T>::drain_prefix(shard_id)
				.map(|(m, _)| {
					MemberShard::<T>::remove(&m);
					m
				})
				.collect::<Vec<_>>();
			T::Elections::shard_offline(network, members);
			Self::deposit_event(Event::ShardOffline(shard_id));
		}
		/// Checks for DKG timeouts and handles shard state transitions accordingly.
		/// # Flow
		///   1. Iterate over the [`DkgTimeout`] storage.
		///   2. Check if the DKG process of any shard has timed out.
		///   3. For timed-out shards, update their status to offline and emit the [`Event::ShardKeyGenTimedOut`] event.
		///   4. Remove DKG timeout entries for shards that are no longer in `Created` or `Committed` states.
		pub(crate) fn timeout_dkgs(n: BlockNumberFor<T>) -> Weight {
			let mut num_timeouts = 0u32;
			// Iterate over DKG timeouts
			DkgTimeout::<T>::drain_prefix(n).for_each(|(shard_id, _)| {
				if let Some(status) = ShardState::<T>::get(shard_id) {
					if matches!(status, ShardStatus::Created | ShardStatus::Committed) {
						Self::remove_shard_offline(shard_id);
						Self::deposit_event(Event::ShardKeyGenTimedOut(shard_id));
						num_timeouts = num_timeouts.saturating_plus_one();
					}
				}
			});
			DkgTimeoutCounter::<T>::remove(n);
			<T as Config>::WeightInfo::timeout_dkgs(num_timeouts)
		}
		/// Fetches all shards associated with a given account.
		/// # Flow
		///   1. Iterate over [`ShardMembers`] storage to find all shards the account is a member of.
		///   2. Collect and return the shard IDs.
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
		/// Retrieves all members of a specified shard.
		/// # Flow
		///   1. Iterate over [`ShardMembers`] storage for the given shard ID.
		///   2. Collect and return the member statuses.
		pub fn get_shard_members(shard_id: ShardId) -> Vec<(AccountId, MemberStatus)> {
			ShardMembers::<T>::iter_prefix(shard_id).collect()
		}
		/// Retrieves the threshold value of a specified shard.
		///
		/// # Flow
		///   1. Retrieve and return the threshold value from [`ShardThreshold`] storage.
		pub fn get_shard_threshold(shard_id: ShardId) -> u16 {
			ShardThreshold::<T>::get(shard_id).unwrap_or_default()
		}
		/// Retrieves the current status of a specified shard.
		/// # Flow
		///   1. Retrieve and return the status from [`ShardState`] storage.
		pub fn get_shard_status(shard_id: ShardId) -> ShardStatus {
			ShardState::<T>::get(shard_id).unwrap_or_default()
		}
		/// Retrieves the commitment of a specified shard, if available.
		///
		/// # Flow
		///   1. Retrieve and return the commitment from [`ShardCommitment`] storage.
		pub fn get_shard_commitment(shard_id: ShardId) -> Option<Commitment> {
			ShardCommitment::<T>::get(shard_id)
		}
	}

	impl<T: Config> ShardsInterface for Pallet<T> {
		/// Updates shard state when a member comes online.
		///
		/// # Flow
		///   1. Retrieves the `shard_id` associated with the member `id`.
		///   2. Retrieves the current old_status of the shard.
		///   3. Increments the count of online members [`ShardMembersOnline`].
		///   4. Updates [`ShardState`] to `Offline` if the previous status was `Created` or `Committed`.
		fn member_online(id: &AccountId, _network: NetworkId) {
			let Some(shard_id) = MemberShard::<T>::get(id) else { return };
			let Some(old_status) = ShardState::<T>::get(shard_id) else { return };
			ShardMembersOnline::<T>::mutate(shard_id, |x| *x = x.saturating_plus_one());
			match old_status {
				ShardStatus::Created | ShardStatus::Committed => {
					ShardState::<T>::insert(shard_id, ShardStatus::Offline)
				},
				_ => (),
			}
		}

		fn members_offline(members: Vec<AccountId>) {
			let mut shard_updates: BTreeMap<ShardId, (u32, Option<ShardStatus>)> = BTreeMap::new();
			let mut shards_to_remove_offline = Vec::new();

			// First pass: Collect updates for each shard
			for member in members {
				let Some(shard_id) = MemberShard::<T>::get(member) else { continue };

				// Get or initialize the shard entry in the update map
				let (online_count, status_update) =
					shard_updates.entry(shard_id).or_insert_with(|| {
						let online = ShardMembersOnline::<T>::get(shard_id);
						let status = ShardState::<T>::get(shard_id);
						(online.into(), status)
					});

				*online_count = online_count.saturating_sub(1);

				// If we haven't checked shard state yet, do it once
				if let Some(old_status) = *status_update {
					let Some(shard_threshold) = ShardThreshold::<T>::get(shard_id) else {
						continue;
					};

					// Determine new status
					let new_status = match old_status {
						ShardStatus::Created | ShardStatus::Committed => ShardStatus::Offline,
						ShardStatus::Online if *online_count < shard_threshold.into() => {
							ShardStatus::Offline
						},
						_ => old_status,
					};

					if matches!(new_status, ShardStatus::Offline)
						&& !matches!(old_status, ShardStatus::Offline)
					{
						shards_to_remove_offline.push(shard_id);
					}

					// Update only if status changes
					if new_status != old_status {
						*status_update = Some(new_status);
					}
				}
			}

			// Batch storage updates
			for (shard_id, (new_online_count, status_update)) in shard_updates {
				ShardMembersOnline::<T>::insert(
					shard_id,
					TryInto::<u16>::try_into(new_online_count).unwrap_or_default(),
				);

				if let Some(new_status) = status_update {
					ShardState::<T>::insert(shard_id, new_status);
				}
			}

			// Remove offline shards in batch
			for shard_id in shards_to_remove_offline {
				Self::remove_shard_offline(shard_id);
			}
		}

		/// Checks if a specified shard is currently online.
		///
		/// # Flow
		///   1. Retrieves the `ShardState` for the given `shard_id`.
		///   2. Returns `true` if the shard status is [`Some(ShardStatus::Online)`], indicating the shard is online; otherwise, returns `false`.
		fn is_shard_online(shard_id: ShardId) -> bool {
			matches!(ShardState::<T>::get(shard_id), Some(ShardStatus::Online))
		}
		/// Checks if a specified account is a member of any shard.
		///
		/// # Flow
		///   1. Retrieves the shard `ID` associated with the member account from [`MemberShard`].
		///   2. Returns `true` if the shard `ID` is present (`Some`), indicating the account is a member; otherwise, returns `false`.
		fn is_shard_member(member: &AccountId) -> bool {
			MemberShard::<T>::get(member).is_some()
		}
		/// Retrieves the network identifier associated with a specified shard.
		///
		/// # Flow
		///   1. Retrieves and returns the network ID stored in [`ShardNetwork`] for the given `shard_id`.
		fn shard_network(shard_id: ShardId) -> Option<NetworkId> {
			ShardNetwork::<T>::get(shard_id)
		}
		/// Retrieves the list of account identifiers that are members of a specified shard.
		///
		/// # Flow
		///   1. Iterates over `ShardMembers` entries with the prefix `shard_id`.
		///   2. Collects and returns the list of account identifiers (`AccountId`) associated with the shard.
		fn shard_members(shard_id: ShardId) -> Vec<AccountId> {
			ShardMembers::<T>::iter_prefix(shard_id).map(|(a, _)| a).collect::<Vec<_>>()
		}
		/// Creates a new shard with specified network, members, and threshold, initializing its state and storing relevant data.
		///
		/// # Flow
		///   1. Generates a new `shard_id` using [`ShardIdCounter`].
		///   2. Stores the network ID in [`ShardNetwork`] for the `shard_id`.
		///   3. Initializes the ShardState to [`ShardStatus::Created`].
		///   4. Sets the creation time in [`DkgTimeout`].
		///   5. Stores the threshold in [`ShardThreshold`].
		///   6. Inserts each member into ShardMembers and associates them with [`MemberStatus::Added`].
		///   7. Registers each member in `MemberShard` with the `shard_id`.
		///   8. Emits a [`Event::ShardCreated`] event with the `shard_id` and network.
		fn create_shard(
			network: NetworkId,
			members: Vec<AccountId>,
			threshold: u16,
		) -> Result<ShardId, DispatchError> {
			let dkg_timeout_block =
				frame_system::Pallet::<T>::block_number().saturating_add(T::DkgTimeout::get());
			let dkg_timeout_counter = <DkgTimeoutCounter<T>>::get(dkg_timeout_block);
			ensure!(
				dkg_timeout_counter
					< <<T as Config>::Elections as ElectionsInterface>::MaxElectionsPerBlock::get(),
				Error::<T>::MaxShardsCreatedThisBlock
			);
			<DkgTimeoutCounter<T>>::insert(
				dkg_timeout_block,
				dkg_timeout_counter.saturating_plus_one(),
			);
			let shard_id = <ShardIdCounter<T>>::get();
			<ShardIdCounter<T>>::put(shard_id.saturating_plus_one());
			<ShardNetwork<T>>::insert(shard_id, network);
			<ShardState<T>>::insert(shard_id, ShardStatus::Created);
			<DkgTimeout<T>>::insert(dkg_timeout_block, shard_id, ());
			<ShardThreshold<T>>::insert(shard_id, threshold);
			for member in &members {
				ShardMembers::<T>::insert(shard_id, member, MemberStatus::Added);
				MemberShard::<T>::insert(member, shard_id);
			}
			ShardMembersOnline::<T>::insert(shard_id, members.len() as u16);
			Self::deposit_event(Event::ShardCreated(shard_id, network));
			Ok(shard_id)
		}
		/// Retrieves the public key of the next signer for the specified shard, updating the signer index.
		///
		/// # Flow
		///   1. Retrieves the list of members (`AccountId`) for the specified `shard_id`.
		///   2. Retrieves the current `signer_index` for the shard.
		///   3. Retrieves the public key of the next signer using `T::Members::member_public_key`.
		///   4. Updates the signer index in [`SignerIndex`].
		///   5. Returns the retrieved public key of the signer.
		fn next_signer(shard_id: ShardId) -> PublicKey {
			let members = Self::get_shard_members(shard_id);
			let signer_index: usize =
				SignerIndex::<T>::get(shard_id).try_into().expect("Checked indexing already");
			let signer = T::Members::member_public_key(&members[signer_index].0)
				.expect("All signers should be registered members");
			let next_signer_index =
				if members.len() as u32 == (signer_index as u32).saturating_plus_one() {
					0
				} else {
					signer_index.saturating_plus_one()
				};
			SignerIndex::<T>::insert(shard_id, next_signer_index as u32);
			signer
		}
		/// Retrieves the TSS public key associated with the specified shard, if available.
		///
		/// # Flow
		///   1. Retrieves the commitment [`Vec<TssPublicKey>`] associated with the `shard_id` from [`ShardCommitment`].
		///   2. Returns the first element of the commitment [`TssPublicKey`] if it exists; otherwise, returns `None`.
		fn tss_public_key(shard_id: ShardId) -> Option<TssPublicKey> {
			ShardCommitment::<T>::get(shard_id).map(|commitment| commitment.0[0])
		}
	}
}
