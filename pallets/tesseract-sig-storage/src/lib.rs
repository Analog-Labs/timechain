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
	use sp_runtime::traits::{AppVerify, Scale};
	use sp_std::{collections::vec_deque::VecDeque, vec::Vec};
	use task_schedule::ScheduleInterface;
	use time_primitives::{
		abstraction::{OCWReportData, OCWSigData, OCWTSSGroupKeyData},
		crypto::Signature,
		sharding::{EligibleShard, HandleShardTasks, IncrementTaskTimeoutCount, Network, Shard},
		KeyId, ScheduleCycle, ShardId as ShardIdType, SignatureData, TimeId, OCW_REP_KEY,
		OCW_SIG_KEY, OCW_TSS_KEY,
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
			Self::ocw_get_tss_data();
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
		/// Minimum reports made by shard reporter to define committed offense
		#[pallet::constant]
		type MinReportsPerCommittedOffense: Get<u8>;
		type TaskScheduleHelper: ScheduleInterface<Self::AccountId>;
		type SessionInterface: SessionInterface<Self::AccountId>;
		#[pallet::constant]
		type MaxChronicleWorkers: Get<u32>;
		type TaskAssigner: HandleShardTasks<u64, Network, u64>;
		/// Maximum number of task execution timeouts before shard is put offline
		#[pallet::constant]
		type MaxTimeouts: Get<u8>;
	}

	#[pallet::storage]
	#[pallet::getter(fn shard_id)]
	/// Counter for creating unique shard_ids during on-chain creation
	pub type ShardId<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Network for which shards can be assigned tasks
	#[pallet::storage]
	#[pallet::getter(fn shard_network)]
	pub type ShardNetwork<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, u64, (), OptionQuery>;

	/// Indicates precise members of each TSS set by it's u64 id
	/// Required for key generation and identification
	#[pallet::storage]
	#[pallet::getter(fn tss_shards)]
	pub type TssShards<T: Config> = StorageMap<_, Blake2_128Concat, u64, ShardState, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tss_group_key)]
	pub type TssGroupKey<T: Config> = StorageMap<_, Blake2_128Concat, u64, [u8; 33], OptionQuery>;

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
	#[pallet::getter(fn reports)]
	pub type Reports<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, TimeId, Blake2_128Concat, TimeId, u8, ValueQuery>;

	/// record the last block number of each chronicle worker commit valid signature
	#[pallet::storage]
	#[pallet::getter(fn last_committed_chronicle)]
	pub type LastCommittedChronicle<T: Config> =
		StorageMap<_, Blake2_128Concat, TimeId, T::BlockNumber, ValueQuery>;

	/// record the last block number of each shard commit valid signature
	#[pallet::storage]
	#[pallet::getter(fn last_committed_shard)]
	pub type LastCommittedShard<T: Config> =
		StorageMap<_, Blake2_128Concat, u64, T::BlockNumber, ValueQuery>;

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
		NewTssGroupKey(u64, [u8; 33]),

		/// Active has been registered with new Id
		/// Eligible for tasks from the Network
		/// .0 ShardId
		/// .1 Network
		ShardRegistered(u64, Network),

		/// Task execution timed out for task
		TaskExecutionTimeout(KeyId),

		/// Shard went offline due to committed offenses preventing threshold
		/// or task execution timeout(s) by collector
		/// .0 ShardId
		ShardOffline(u64),

		/// Offense reported by reporter
		/// .0 ShardId
		/// .1 Offender
		/// .2 Reporter
		OffenseReported(u64, TimeId, TimeId),

		/// Chronicle has been registered
		/// .0 TimeId
		/// .1 Validator's AccountId
		ChronicleRegistered(TimeId, T::AccountId),

		/// Task claimed for shard
		/// .0 Network
		/// .1 ShardId
		/// .2 TaskId
		TaskClaimedForShard(Network, u64, u64),
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
		ShardOffline,

		/// Caller is not shard's collector so cannot call this function
		OnlyCallableByCollector,
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

			let shard_id: u64 = T::TaskScheduleHelper::get_assigned_shard_for_key(key_id)?;

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
			T::TaskScheduleHelper::decrement_schedule_cycle(key_id)?;
			T::TaskScheduleHelper::update_completed_task(key_id);

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
			shard_id: ShardIdType,
			group_key: [u8; 33],
			proof: Signature,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			let shard_state =
				<TssShards<T>>::get(shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;
			let collector: &sp_runtime::AccountId32 = shard_state.shard.collector();

			let raw_public_key: &[u8; 32] = collector.as_ref();
			let collector_public_id =
				sp_application_crypto::sr25519::Public::from_raw(*raw_public_key);

			ensure!(
				proof.verify(group_key.as_ref(), &collector_public_id.into()),
				Error::<T>::InvalidValidationSignature
			);

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
			let shard_id = <ShardId<T>>::get();
			// compute next ShardId before putting it in storage
			let next_shard_id = shard_id.checked_add(1u64).ok_or(Error::<T>::ShardIdOverflow)?;
			<ShardNetwork<T>>::insert(net, shard_id, ());
			<TssShards<T>>::insert(shard_id, shard);
			<ShardId<T>>::put(next_shard_id);
			Self::deposit_event(Event::ShardRegistered(shard_id, net));
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

		/// Collector-only can report shard members for not submitting signatures
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::report_misbehavior())]
		pub fn report_misbehavior(
			origin: OriginFor<T>,
			shard_id: u64,
			offender: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let mut shard_state =
				<TssShards<T>>::get(shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;
			let (offender, reporter) =
				shard_state.increment_committed_offense_count::<T>(offender, caller, shard_id)?;
			TssShards::<T>::insert(shard_id, shard_state);
			Self::deposit_event(Event::OffenseReported(shard_id, offender, reporter));
			Ok(())
		}

		/// Method for root to set shard offline and reassign tasks
		/// It is used in collector offline and for testing
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::force_set_shard_offline())]
		pub fn force_set_shard_offline(origin: OriginFor<T>, shard_id: u64) -> DispatchResult {
			ensure_root(origin)?;
			let mut on_chain_shard_state =
				<TssShards<T>>::get(shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;

			if on_chain_shard_state.is_online() {
				on_chain_shard_state.status = ShardStatus::Offline;
				// Handle all of this shard's tasks
				T::TaskAssigner::handle_shard_tasks(shard_id, on_chain_shard_state.network);
				<TssShards<T>>::insert(shard_id, on_chain_shard_state);
				Self::deposit_event(Event::ShardOffline(shard_id));
				Ok(())
			} else {
				Err(Error::<T>::ShardOffline.into())
			}
		}

		/// Extrinsic for shard collector to claim task for shard
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::force_set_shard_offline())] // TODO: add benchmark
		pub fn claim_task(origin: OriginFor<T>, shard_id: u64, task_id: u64) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let shard_state =
				<TssShards<T>>::get(shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;
			ensure!(shard_state.is_online(), Error::<T>::ShardOffline);
			ensure!(
				shard_state.shard.is_collector(&account_to_time_id::<T::AccountId>(caller)),
				Error::<T>::OnlyCallableByCollector
			);
			T::TaskAssigner::claim_task_for_shard(shard_id, shard_state.network, task_id)?;

			Self::deposit_event(Event::TaskClaimedForShard(shard_state.network, shard_id, task_id));
			Ok(())
		}
	}

	impl<T: Config> IncrementTaskTimeoutCount<u64> for Pallet<T> {
		fn increment_task_timeout_count(id: u64) {
			if let Some(mut shard_state) = TssShards::<T>::get(id) {
				shard_state.increment_task_timeout_count::<T>(id);
			}
		}
	}

	impl<T: Config> EligibleShard<u64, Network> for Pallet<T> {
		fn is_eligible_shard(id: u64) -> bool {
			if let Some(shard_state) = <TssShards<T>>::get(id) {
				shard_state.is_online()
			} else {
				false
			}
		}
		fn next_eligible_shard(network: Network) -> Option<u64> {
			let mut least_assigned_shard: Option<u64> = None;
			let mut lowest_assigned_shard_count = 10_000usize;
			for (shard_id, _) in <ShardNetwork<T>>::iter_prefix(network) {
				if Self::is_eligible_shard(shard_id) {
					let shard_schedule_count =
						T::TaskScheduleHelper::get_assigned_schedule_count(shard_id);
					if lowest_assigned_shard_count > shard_schedule_count {
						lowest_assigned_shard_count = shard_schedule_count;
						least_assigned_shard = Some(shard_id);
					}
				}
			}
			least_assigned_shard
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_offense_count(offender: &TimeId) -> u8 {
			Reports::<T>::iter_prefix(offender)
				.fold(0u8, |acc, (_, count)| acc.saturating_add(count))
		}
		pub fn get_offense_count_for_reporter(offender: &TimeId, reporter: &TimeId) -> u8 {
			Reports::<T>::get(offender, reporter)
		}
		pub fn active_shards(network: Network) -> Vec<(u64, Shard)> {
			<TssShards<T>>::iter()
				.filter(|(_, s)| s.is_online() && s.network == network)
				.map(|(id, s)| (id, s.shard))
				.collect()
		}
		pub fn inactive_shards(network: Network) -> Vec<(u64, Shard)> {
			<TssShards<T>>::iter()
				.filter(|(_, s)| !s.is_online() && s.network == network)
				.map(|(id, s)| (id, s.shard))
				.collect()
		}
		// Getter method for runtime api storage access
		pub fn api_tss_shards() -> Vec<(u64, Shard)> {
			<TssShards<T>>::iter().map(|(id, state)| (id, state.shard)).collect()
		}

		fn ocw_get_tss_data() {
			let storage_ref = StorageValueRef::persistent(OCW_TSS_KEY);

			const EMPTY_DATA: () = ();

			let outer_res = storage_ref.mutate(
				|res: Result<Option<VecDeque<Vec<u8>>>, StorageRetrievalError>| {
					match res {
						Ok(Some(mut data)) => {
							// iteration batch of 5
							for _ in 0..2 {
								let Some(tss_req_vec) = data.pop_front() else{
									break;
								};

								let Ok(tss_req) = OCWTSSGroupKeyData::decode(&mut tss_req_vec.as_slice()) else {
									continue;
								};

								if let Err(err) = Self::ocw_submit_tss_group_key(tss_req.clone()) {
									log::error!(
										"Error occured while submitting extrinsic {:?}",
										err
									);
								};

								log::info!("Submitting OCW TSS key");
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
					log::info!("TSS OCW tss is empty");
				},
				Err(MutateStorageError::ConcurrentModification(_)) => {
					log::error!("💔 Error updating local storage in TSS OCW Signature",);
				},
				Ok(_) => {},
			}
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

								log::info!("Submitting OCW Sig Data");
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
								log::info!("Submitting OCW Report Data");
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

		fn ocw_submit_tss_group_key(data: OCWTSSGroupKeyData) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();

			if let Some((acc, res)) =
				signer.send_signed_transaction(|_account| Call::submit_tss_group_key {
					shard_id: data.shard_id,
					group_key: data.group_key,
					proof: data.proof.clone(),
				}) {
				if res.is_err() {
					log::error!("failure: offchain_tss_tx: tx sent: {:?}", acc.id);
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
