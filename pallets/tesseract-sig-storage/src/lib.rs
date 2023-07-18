#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod crypto {
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	use time_primitives::SIG_KEY_TYPE;
	app_crypto!(sr25519, SIG_KEY_TYPE);
	pub struct SigAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for SigAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for SigAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

pub mod shard;
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use crate::shard::*;
	use frame_support::{
		pallet_prelude::{ValueQuery, *},
		storage::bounded_vec::BoundedVec,
		traits::Time,
	};
	use frame_system::offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
	};
	use frame_system::pallet_prelude::*;
	use pallet_staking::SessionInterface;
	use scale_info::StaticTypeInfo;
	use sp_runtime::offchain::storage::{
		MutateStorageError, StorageRetrievalError, StorageValueRef,
	};
	use sp_runtime::{
		traits::{AppVerify, Scale},
		SaturatedConversion, Saturating,
	};
	use sp_std::{
		collections::{btree_set::BTreeSet, vec_deque::VecDeque},
		result,
		vec::Vec,
	};
	use task_schedule::ScheduleInterface;
	use time_primitives::{
		abstraction::{OCWReportData, OCWSigData},
		crypto::Signature,
		inherents::{InherentError, TimeTssKey, INHERENT_IDENTIFIER},
		sharding::{
			EligibleShard, IncrementTaskTimeoutCount, Network, ReassignShardTasks, Shard, ShardId,
			ShardPublicKey, DEFAULT_SHARD_PUBLIC_KEY,
		},
		KeyId, ScheduleCycle, SignatureData, TimeId, OCW_REP_KEY, OCW_SIG_KEY,
	};

	pub trait WeightInfo {
		fn store_signature(_s: u32) -> Weight;
		fn submit_tss_group_key(_s: u32) -> Weight;
		fn register_shard() -> Weight;
		fn register_chronicle() -> Weight;
		fn report_misbehavior() -> Weight;
		fn force_set_shard_offline() -> Weight;
	}

	impl WeightInfo for () {
		fn store_signature(_s: u32) -> Weight {
			Weight::from_parts(0, 1)
		}
		fn submit_tss_group_key(_s: u32) -> Weight {
			Weight::from_parts(0, 1)
		}
		fn register_shard() -> Weight {
			Weight::from_parts(0, 1)
		}
		fn register_chronicle() -> Weight {
			Weight::from_parts(0, 1)
		}
		fn report_misbehavior() -> Weight {
			Weight::from_parts(0, 1)
		}
		fn force_set_shard_offline() -> Weight {
			Weight::from_parts(0, 1)
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(_block_number: T::BlockNumber) {
			Self::ocw_get_sig_data();
			Self::ocw_get_report_data();
		}
	}

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Moment: Parameter
			+ Default
			+ Scale<Self::BlockNumber, Output = Self::Moment>
			+ Copy
			+ MaxEncodedLen
			+ StaticTypeInfo;
		type Timestamp: Time<Moment = Self::Moment>;
		/// Slashing percentage for commiting misbehavior
		#[pallet::constant]
		type SlashingPercentage: Get<u8>;
		/// Slashing threshold percentage for commiting misbehavior consensus
		#[pallet::constant]
		type SlashingPercentageThreshold: Get<u8>;
		type TaskScheduleHelper: ScheduleInterface<Self::AccountId, Self::BlockNumber>;
		type SessionInterface: SessionInterface<Self::AccountId>;
		#[pallet::constant]
		type MaxChronicleWorkers: Get<u32>;
		type TaskAssigner: ReassignShardTasks<ShardId>;
		/// Maximum number of task execution timeouts before shard is put offline
		#[pallet::constant]
		type MaxTimeouts: Get<u8>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_shards_index)]
	/// Counter for getting (N) next available shard(s)s
	pub type GetShardsIndex<T: Config> = StorageValue<_, ShardId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_id_counter)]
	/// Counter for creating unique shard_ids during on-chain creation
	pub type ShardIdCounter<T: Config> = StorageValue<_, ShardId, ValueQuery>;

	/// Indicates precise members of each TSS set by it's u64 id
	/// Required for key generation and identification
	#[pallet::storage]
	#[pallet::getter(fn tss_shards)]
	pub type TssShards<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, ShardState, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tss_group_key)]
	pub type TssGroupKey<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, ShardPublicKey, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn signature_storage)]
	pub type SignatureStoreData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		KeyId,
		Blake2_128Concat,
		ScheduleCycle,
		SignatureData,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn reported_offences)]
	pub type ReportedOffences<T: Config> =
		StorageMap<_, Blake2_128Concat, TimeId, (u8, BTreeSet<TimeId>), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn commited_offences)]
	pub type CommitedOffences<T: Config> =
		StorageMap<_, Blake2_128Concat, TimeId, (u8, BTreeSet<TimeId>), OptionQuery>;

	/// record the last block number of each chronicle worker commit valid signature
	#[pallet::storage]
	#[pallet::getter(fn last_committed_chronicle)]
	pub type LastCommittedChronicle<T: Config> =
		StorageMap<_, Blake2_128Concat, TimeId, T::BlockNumber, ValueQuery>;

	/// record the last block number of each shard commit valid signature
	#[pallet::storage]
	#[pallet::getter(fn last_committed_shard)]
	pub type LastCommittedShard<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, T::BlockNumber, ValueQuery>;

	/// record the chronicle worker ids for each validator
	#[pallet::storage]
	#[pallet::getter(fn validator_to_chronicle)]
	pub type ValidatorToChronicle<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<TimeId, T::MaxChronicleWorkers>>;

	/// record the chronicle worker's owner or its validator account id
	#[pallet::storage]
	#[pallet::getter(fn chronicle_owner)]
	pub type ChronicleOwner<T: Config> =
		StorageMap<_, Blake2_128Concat, TimeId, T::AccountId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the signature id that uniquely identify the signature
		SignatureStored(KeyId, ScheduleCycle),

		/// New group key submitted to runtime
		/// .0 - ShardId
		/// .1 - group key bytes
		NewTssGroupKey(ShardId, ShardPublicKey),

		/// Shard has ben registered with new Id
		/// .0 ShardId
		ShardRegistered(ShardId),

		/// Task execution timed out for task
		TaskExecutionTimeout(KeyId),

		/// Shard went offline due to committed offenses preventing threshold
		/// or task execution timeout(s) by collector
		/// .0 ShardId
		ShardOffline(ShardId),

		/// Offence reported, above threshold s.t.
		/// reports are moved from reported to committed.
		/// .0 Offender TimeId
		/// .1 Report count
		OffenceCommitted(TimeId, u8),

		/// Offence reported
		/// .0 Offender TimeId
		/// .1 Report count
		OffenceReported(TimeId, u8),

		/// Chronicle has ben registered
		/// .0 TimeId
		/// .1 Validator's AccountId
		ChronicleRegistered(TimeId, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Shard registartion failed because wrong number of members
		/// NOTE: supported sizes are 3, 5, and 10
		UnsupportedMembershipSize,

		DuplicateShardMembersNotAllowed,

		/// Encoded account wrong length
		EncodedAccountWrongLen,

		/// Default account is not allowed for this operation
		DefaultAccountForbidden,

		/// Unauthorized attempt to add signed data
		UnregisteredWorkerDataSubmission,

		/// Reporter TimeId can not be converted to Public key
		InvalidReporterId,

		/// Offender not in members
		OffenderNotInMembers,

		/// Cannot set collector if they are already in that role
		AlreadyCollector,

		/// Shard does not exist in storage
		ShardIsNotRegistered,

		/// Misbehavior report proof verification failed
		ProofVerificationFailed,

		/// ShardId generation overflowed u64 type
		ShardIdOverflow,

		/// Collector index exceeds length of members
		CollectorIndexBeyondMemberLen,

		/// Invalid Caller,
		InvalidCaller,

		/// Task not scheduled
		TaskNotScheduled,

		/// Invalid validation signature
		InvalidValidationSignature,

		///TSS Signature already added
		DuplicateSignature,

		/// Chronicle already registered
		ChronicleAlreadyRegistered,

		/// Failed to get validator id
		FailedToGetValidatorId,

		/// Only validator can register chronicle
		OnlyValidatorCanRegisterChronicle,

		/// Chronicle already in set
		ChronicleAlreadyInSet,

		/// Chronicle set is full
		ChronicleSetIsFull,

		/// Chronicle not registered
		ChronicleNotRegistered,

		///Offchain signed tx failed
		OffchainSignedTxFailed,

		///no local account for signed tx
		NoLocalAcctForSignedTx,

		/// Shard status is offline now
		ShardAlreadyOffline,

		/// Caller is not shard's collector so cannot call this function
		OnlyCallableByCollector,
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			if let Ok(inherent_data) = data.get_data::<TimeTssKey>(&INHERENT_IDENTIFIER) {
				return match inherent_data {
					None => None,
					Some(inherent_data) if inherent_data.group_key != DEFAULT_SHARD_PUBLIC_KEY => {
						// We don't need to set the inherent data every block, it is only needed
						// once.
						let pubk = <TssGroupKey<T>>::get(inherent_data.shard_id);
						if pubk.is_none() {
							Some(Call::submit_tss_group_key {
								shard_id: inherent_data.shard_id,
								group_key: inherent_data.group_key,
							})
						} else {
							None
						}
					},
					_ => None,
				};
			}
			None
		}

		fn check_inherent(
			call: &Self::Call,
			data: &InherentData,
		) -> result::Result<(), Self::Error> {
			let (shard_id, group_key) = match call {
				Call::submit_tss_group_key { shard_id, group_key } => (shard_id, group_key),
				_ => return Err(InherentError::WrongInherentCall),
			};

			let expected_data = data
				.get_data::<TimeTssKey>(&INHERENT_IDENTIFIER)
				.expect("Inherent data is not correctly encoded");

			let Some(expected_data) = expected_data else {
				return Ok(())
			};

			if &expected_data.shard_id != shard_id && &expected_data.group_key != group_key {
				return Err(InherentError::InvalidGroupKey(TimeTssKey {
					group_key: *group_key,
					shard_id: *shard_id,
				}));
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::submit_tss_group_key { .. })
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::store_signature(1))]
		pub fn store_signature(
			origin: OriginFor<T>,
			auth_sig: Signature,
			signature_data: SignatureData,
			key_id: KeyId,
			schedule_cycle: ScheduleCycle,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let mut is_recurring = false;
			let schedule_data = T::TaskScheduleHelper::get_schedule_via_key(key_id)?;
			let payable_schedule_data =
				T::TaskScheduleHelper::get_payable_schedule_via_key(key_id)?;

			let shard_id = if let Some(schedule) = schedule_data {
				is_recurring = schedule.cycle > 1;
				schedule.shard_id
			} else if let Some(payable_schedule) = payable_schedule_data {
				payable_schedule.shard_id
			} else {
				return Err(Error::<T>::TaskNotScheduled.into());
			};

			let shard_state =
				<TssShards<T>>::get(shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;
			let collector = shard_state.shard.collector();

			let raw_public_key: &[u8; 32] = collector.as_ref();
			let collector_public_id =
				sp_application_crypto::sr25519::Public::from_raw(*raw_public_key);

			ensure!(
				auth_sig.verify(signature_data.as_ref(), &collector_public_id.into()),
				Error::<T>::InvalidValidationSignature
			);

			ensure!(
				<SignatureStoreData<T>>::get(key_id, schedule_cycle).is_none(),
				Error::<T>::DuplicateSignature
			);

			// Updates completed task status and start_execution_block for
			// ongoing recurring tasks.
			T::TaskScheduleHelper::update_completed_task(key_id);
			if is_recurring {
				T::TaskScheduleHelper::decrement_schedule_cycle(key_id)?;
			}

			<SignatureStoreData<T>>::insert(key_id, schedule_cycle, signature_data);

			Self::deposit_event(Event::SignatureStored(key_id, schedule_cycle));
			<LastCommittedChronicle<T>>::insert(
				collector,
				frame_system::Pallet::<T>::block_number(),
			);
			<LastCommittedShard<T>>::insert(shard_id, frame_system::Pallet::<T>::block_number());
			Ok(())
		}

		/// Submits TSS group key to runtime
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_tss_group_key(1))]
		pub fn submit_tss_group_key(
			origin: OriginFor<T>,
			shard_id: ShardId,
			group_key: ShardPublicKey,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			<TssGroupKey<T>>::insert(shard_id, group_key);
			<TssShards<T>>::try_mutate(shard_id, |shard_state| -> DispatchResult {
				let details = shard_state.as_mut().ok_or(Error::<T>::ShardIsNotRegistered)?;
				details.status = ShardStatus::Online;
				Ok(())
			})?;
			Self::deposit_event(Event::NewTssGroupKey(shard_id, group_key));

			Ok(().into())
		}

		/// Root can register new shard via providing
		/// set of IDs matching one of supported size of shard
		/// # Param
		/// * members - supported sized set of shard members Id
		/// * collector - index of collector if not index 0
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::register_shard())]
		pub fn register_shard(
			origin: OriginFor<T>,
			members: Vec<TimeId>,
			collector_index: Option<u8>,
			net: Network,
		) -> DispatchResult {
			ensure_root(origin)?;
			// ensure each member is registered and no repeated members
			let mut members_dedup = Vec::new();
			for member in members.iter() {
				ensure!(
					!members_dedup.contains(&member),
					Error::<T>::DuplicateShardMembersNotAllowed
				);
				ensure!(
					ChronicleOwner::<T>::contains_key(member.clone()),
					Error::<T>::ChronicleNotRegistered
				);
				// do not push to vector for last member
				if members_dedup.len() < members.len() - 1 {
					members_dedup.push(member);
				}
			}
			let shard = ShardState::new::<T>(members.clone(), collector_index, net)?;
			// get unused ShardId from storage
			let shard_id = <ShardIdCounter<T>>::get();
			// compute next ShardId before putting it in storage
			let next_shard_id = shard_id.checked_add(1).ok_or(Error::<T>::ShardIdOverflow)?;
			<TssShards<T>>::insert(shard_id, shard);
			<ShardIdCounter<T>>::put(next_shard_id);
			Self::deposit_event(Event::ShardRegistered(shard_id));
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::register_chronicle())]
		pub fn register_chronicle(origin: OriginFor<T>, member: TimeId) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// ensure chronicle is not already registered
			ensure!(
				!ChronicleOwner::<T>::contains_key(member.clone()),
				Error::<T>::ChronicleAlreadyRegistered
			);

			// get current validator set
			let validator_set = T::SessionInterface::validators();

			// caller must be one of validators
			ensure!(validator_set.contains(&caller), Error::<T>::OnlyValidatorCanRegisterChronicle);

			// update chronicle worker set for caller
			ValidatorToChronicle::<T>::try_mutate(caller.clone(), |chronicles| match chronicles {
				Some(ref mut node) => {
					if node.contains(&member) {
						return Err::<(), Error<T>>(Error::<T>::ChronicleAlreadyInSet);
					};

					node.try_insert(0, member.clone())
						.map_err(|_| Error::<T>::ChronicleSetIsFull)?;
					Ok(())
				},
				None => {
					let mut a = BoundedVec::<TimeId, T::MaxChronicleWorkers>::default();
					let _ = a.try_insert(0, member.clone());
					*chronicles = Some(a);
					Ok(())
				},
			})?;

			ChronicleOwner::<T>::insert(&member, caller.clone());
			Self::deposit_event(Event::ChronicleRegistered(member, caller));
			Ok(())
		}

		/// Method to provide misbehavior report to runtime
		/// Is protected with proven ownership of private key to prevent spam
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::report_misbehavior())]
		pub fn report_misbehavior(
			origin: OriginFor<T>,
			shard_id: ShardId,
			offender: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let mut shard_state =
				<TssShards<T>>::get(shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;
			fn account_to_time_id<A: Encode>(account_id: A) -> TimeId {
				account_id.encode()[..].try_into().unwrap()
			}
			let (reporter, offender) = (
				account_to_time_id::<T::AccountId>(caller),
				account_to_time_id::<T::AccountId>(offender),
			);
			ensure!(shard_state.shard.is_collector(&reporter), Error::<T>::OnlyCallableByCollector);
			ensure!(shard_state.shard.contains_member(&offender), Error::<T>::OffenderNotInMembers);
			let reported_offences_count =
				if let Some(mut known_offender) = <ReportedOffences<T>>::get(&offender) {
					// increment report count
					let new_report_count = known_offender.0.saturating_plus_one();
					// update offender report count
					known_offender.0 = new_report_count;
					// temporary report threshold while only collector can make reports
					// => 2 reports is sufficient to lead to committed offenses
					const REPORT_THRESHOLD: usize = 2;
					if new_report_count.saturated_into::<usize>() >= REPORT_THRESHOLD {
						// increment committed offense count and update state in storage
						shard_state.increment_committed_offense_count::<T>(shard_id);
						// move ReportedOffenses to CommittedOffenses
						<CommitedOffences<T>>::insert(&offender, known_offender);
						// removed ReportedOffences because moved to CommittedOffences
						<ReportedOffences<T>>::remove(&offender);
						Self::deposit_event(Event::OffenceCommitted(
							offender.clone(),
							new_report_count,
						));
					} else {
						<ReportedOffences<T>>::insert(&offender, known_offender);
					}
					new_report_count
				} else if let Some(mut guilty_offender) = <CommitedOffences<T>>::get(&offender) {
					// do allow new reports but only write to `CommittedOffences`
					// (better to allow additional reports than enforce only up to threshold)
					let new_report_count = guilty_offender.0.saturating_plus_one();
					// update known offender report count
					guilty_offender.0 = new_report_count;
					<CommitedOffences<T>>::insert(&offender, guilty_offender);
					new_report_count
				} else {
					// else write first first report ever to ReportedOffences
					let mut new_reports = BTreeSet::new();
					new_reports.insert(reporter);
					let new_report_count = 1u8;
					// insert new report
					<ReportedOffences<T>>::insert(&offender, (new_report_count, new_reports));
					new_report_count
				};
			Self::deposit_event(Event::OffenceReported(offender, reported_offences_count));
			Ok(())
		}

		/// Method for root to set shard offline and reassign tasks
		/// It is used in collector offline and for testing
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::force_set_shard_offline())]
		pub fn force_set_shard_offline(origin: OriginFor<T>, shard_id: ShardId) -> DispatchResult {
			ensure_root(origin)?;
			let mut on_chain_shard_state =
				<TssShards<T>>::get(shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;

			if on_chain_shard_state.is_online() {
				on_chain_shard_state.status = ShardStatus::Offline;
				<TssShards<T>>::mutate(shard_id, |shard_state| {
					*shard_state = Some(on_chain_shard_state)
				});
				T::TaskAssigner::reassign_shard_tasks(shard_id);
				Self::deposit_event(Event::ShardOffline(shard_id));
				Ok(())
			} else {
				Err(Error::<T>::ShardAlreadyOffline.into())
			}
		}
	}

	impl<T: Config> IncrementTaskTimeoutCount<ShardId> for Pallet<T> {
		fn increment_task_timeout_count(id: ShardId) {
			if let Some(mut shard_state) = TssShards::<T>::get(id) {
				shard_state.increment_task_timeout_count::<T>(id);
			}
		}
	}

	impl<T: Config> EligibleShard<ShardId, Network> for Pallet<T> {
		fn is_eligible_shard(id: ShardId) -> bool {
			if let Some(shard_state) = <TssShards<T>>::get(id) {
				shard_state.is_online()
			} else {
				false
			}
		}
		fn is_eligible_shard_for_network(id: ShardId, net: Network) -> bool {
			if let Some(shard_state) = <TssShards<T>>::get(id) {
				shard_state.is_online() && shard_state.net == net
			} else {
				false
			}
		}
		fn get_eligible_shards(id: ShardId, n: usize) -> Vec<ShardId> {
			let net = if let Some(ShardState { net, .. }) = <TssShards<T>>::get(id) {
				net
			} else {
				return Vec::new();
			};
			let mut n_shards = Vec::new();
			let mut shard_id = <GetShardsIndex<T>>::take();
			let max_shard_id = <ShardIdCounter<T>>::get().saturating_sub(1);
			while n_shards.len() < n {
				if Self::is_eligible_shard_for_network(shard_id, net) {
					n_shards.push(shard_id);
				}
				shard_id = if shard_id >= max_shard_id {
					// saturating wrap at max shard_id registered
					0
				} else {
					shard_id + 1
				};
			}
			<GetShardsIndex<T>>::put(shard_id);
			n_shards
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn active_shards() -> Vec<(ShardId, Shard)> {
			<TssShards<T>>::iter()
				.filter(|(_, s)| s.is_online())
				.map(|(id, state)| (id, state.shard))
				.collect()
		}
		pub fn inactive_shards() -> Vec<(ShardId, Shard)> {
			<TssShards<T>>::iter()
				.filter(|(_, s)| !s.is_online())
				.map(|(id, state)| (id, state.shard))
				.collect()
		}
		// Getter method for runtime api storage access
		pub fn api_tss_shards() -> Vec<(ShardId, Shard)> {
			<TssShards<T>>::iter().map(|(id, state)| (id, state.shard)).collect()
		}

		fn ocw_get_sig_data() {
			let storage_ref = StorageValueRef::persistent(OCW_SIG_KEY);

			const EMPTY_DATA: () = ();

			let outer_res = storage_ref.mutate(
				|res: Result<Option<VecDeque<Vec<u8>>>, StorageRetrievalError>| {
					match res {
						Ok(Some(mut data)) => {
							// iteration batch of 5
							for _ in 0..5 {
								let Some(sig_req_vec) = data.pop_front() else{
									break;
								};

								let Ok(sig_req) = OCWSigData::decode(&mut sig_req_vec.as_slice()) else {
									continue;
								};

								if let Err(err) = Self::ocw_submit_signature(sig_req.clone()) {
									log::error!(
										"Error occured while submitting extrinsic {:?}",
										err
									);
								};
							}
							Ok(data)
						},
						Ok(None) => Err(EMPTY_DATA),
						Err(_) => Err(EMPTY_DATA),
					}
				},
			);

			match outer_res {
				Err(MutateStorageError::ValueFunctionFailed(EMPTY_DATA)) => {
					log::info!("TSS OCW Sig is empty");
				},
				Err(MutateStorageError::ConcurrentModification(_)) => {
					log::error!("💔 Error updating local storage in TSS OCW Signature",);
				},
				Ok(_) => {},
			}
		}

		fn ocw_get_report_data() {
			let storage_ref = StorageValueRef::persistent(OCW_REP_KEY);

			const EMPTY_DATA: () = ();

			let outer_res = storage_ref.mutate(
				|res: Result<Option<VecDeque<Vec<u8>>>, StorageRetrievalError>| {
					match res {
						Ok(Some(mut data)) => {
							// iteration batch of 5
							for _ in 0..5 {
								let Some(rep_req_vec) = data.pop_front() else{
									break;
								};

								let Ok(rep_req) = OCWReportData::decode(&mut rep_req_vec.as_slice()) else {
									continue;
								};

								if let Err(err) = Self::ocw_submit_report(rep_req.clone()) {
									log::error!(
										"Error occured while submitting extrinsic {:?}",
										err
									);
								};
							}
							Ok(data)
						},
						Ok(None) => Err(EMPTY_DATA),
						Err(_) => Err(EMPTY_DATA),
					}
				},
			);

			match outer_res {
				Err(MutateStorageError::ValueFunctionFailed(EMPTY_DATA)) => {
					log::info!("TSS OCW Report is empty");
				},
				Err(MutateStorageError::ConcurrentModification(_)) => {
					log::error!("💔 Error updating local storage in TSS OCW Report",);
				},
				Ok(_) => {},
			}
		}

		fn ocw_submit_report(data: OCWReportData) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();

			let offender_id = T::AccountId::decode(&mut data.offender.as_ref()).unwrap();

			if let Some((acc, res)) =
				signer.send_signed_transaction(|_account| Call::report_misbehavior {
					shard_id: data.shard_id,
					offender: offender_id.clone(),
				}) {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
					return Err(Error::OffchainSignedTxFailed);
				} else {
					return Ok(());
				}
			}

			log::error!("No local account available");
			Err(Error::NoLocalAcctForSignedTx)
		}

		fn ocw_submit_signature(data: OCWSigData) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();

			if let Some((acc, res)) =
				signer.send_signed_transaction(|_account| Call::store_signature {
					auth_sig: data.auth_sig.clone(),
					signature_data: data.sig_data,
					key_id: data.key_id,
					schedule_cycle: data.schedule_cycle,
				}) {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
					return Err(Error::OffchainSignedTxFailed);
				} else {
					return Ok(());
				}
			}

			log::error!("No local account available");
			Err(Error::NoLocalAcctForSignedTx)
		}
	}
}
