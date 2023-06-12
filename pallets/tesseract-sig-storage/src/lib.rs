#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

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
		traits::{Time, ValidatorSet},
	};
	use frame_system::pallet_prelude::*;
	use scale_info::StaticTypeInfo;
	use sp_application_crypto::ByteArray;
	use sp_runtime::{
		traits::{AppVerify, Convert, Scale},
		Percent, SaturatedConversion, Saturating,
	};
	use sp_std::{collections::btree_set::BTreeSet, result, vec::Vec};
	use task_schedule::ScheduleFetchInterface;
	use time_primitives::{
		abstraction::ObjectId,
		crypto::{Public, Signature},
		inherents::{InherentError, TimeTssKey, INHERENT_IDENTIFIER},
		sharding::Shard,
		ForeignEventId, SignatureData, TimeId,
	};

	pub trait WeightInfo {
		fn store_signature(_s: u32) -> Weight;
		fn submit_tss_group_key(_s: u32) -> Weight;
		fn register_shard() -> Weight;
		fn register_chronicle() -> Weight;
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
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// pub trait Config: frame_system::Config + pallet_session::Config {

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

		type TaskScheduleHelper: ScheduleFetchInterface<Self::AccountId>;
		type ValidatorSet: ValidatorSet<Self::AccountId>;
		#[pallet::constant]
		type MaxChronicleWorkers: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn shard_id)]
	/// Counter for creating unique shard_ids during on-chain creation
	pub type ShardId<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Indicates precise members of each TSS set by it's u64 id
	/// Required for key generation and identification
	#[pallet::storage]
	#[pallet::getter(fn tss_shards)]
	pub type TssShards<T: Config> = StorageMap<_, Blake2_128Concat, u64, Shard, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tss_group_key)]
	pub type TssGroupKey<T: Config> = StorageMap<_, Blake2_128Concat, u64, [u8; 33], OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn signature_storage)]
	pub type SignatureStoreData<T: Config> =
		StorageMap<_, Blake2_128Concat, ForeignEventId, BTreeSet<SignatureData>, ValueQuery>;

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
		StorageMap<_, Blake2_128Concat, u64, T::BlockNumber, ValueQuery>;

	/// record the chronicle worker ids for each validator
	#[pallet::storage]
	#[pallet::getter(fn validator_to_chronicle)]
	pub type ValidatorToChronicle<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<TimeId, T::MaxChronicleWorkers>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the signature id that uniquely identify the signature
		SignatureStored(ForeignEventId),

		/// New group key submitted to runtime
		/// .0 - set_id,
		/// .1 - group key bytes
		NewTssGroupKey(u64, [u8; 33]),

		/// Shard has ben registered with new Id
		ShardRegistered(u64),

		/// Offence reported, above threshold s.t.
		/// reports are moved from reported to committed.
		/// .0 Offender TimeId
		/// .1 Report count
		OffenceCommitted(TimeId, u8),

		/// Offence reported
		/// .0 Offender TimeId
		/// .1 Report count
		OffenceReported(TimeId, u8),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Shard registartion failed because wrong number of members
		/// NOTE: supported sizes are 3, 5, and 10
		UnsupportedMembershipSize,

		/// Encoded account wrong length
		EncodedAccountWrongLen,

		/// Default account is not allowed for this operation
		DefaultAccountForbidden,

		/// Unauthorized attempt to add signed data
		UnregisteredWorkerDataSubmission,

		/// Reporter TimeId can not be converted to Public key
		InvalidReporterId,

		/// Reporter or offender not in members
		ReporterOrOffenderNotInMembers,

		/// Cannot set collector if they are already in that role
		AlreadyCollector,

		/// Shard does not exist in storage
		ShardIsNotRegistered,

		/// Misbehavior report proof verification failed
		ProofVerificationFailed,

		/// Do not allow more than one misbehavior report of offender by member
		MaxOneReportPerMember,

		/// ShardId generation overflowed u64 type
		ShardIdOverflow,

		/// Collector index exceeds length of members
		CollectorIndexBeyondMemberLen,

		/// Invalid Caller,
		InvalidCaller,

		/// Task not scheduled
		TaskNotScheduled,

		/// Invalid collector id
		InvalidCollectorId,

		///TSS Signature already added
		DuplicateSignature,
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
					Some(inherent_data) if inherent_data.group_key != [0u8; 33] => {
						// We don't need to set the inherent data every block, it is only needed
						// once.
						let pubk = <TssGroupKey<T>>::get(inherent_data.set_id);
						if pubk.is_none() {
							Some(Call::submit_tss_group_key {
								set_id: inherent_data.set_id,
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
			let (set_id, group_key) = match call {
				Call::submit_tss_group_key { set_id, group_key } => (set_id, group_key),
				_ => return Err(InherentError::WrongInherentCall),
			};

			let expected_data = data
				.get_data::<TimeTssKey>(&INHERENT_IDENTIFIER)
				.expect("Inherent data is not correctly encoded")
				.expect("Inherent data must be provided");

			if &expected_data.set_id != set_id && &expected_data.group_key != group_key {
				return Err(InherentError::InvalidGroupKey(TimeTssKey {
					group_key: *group_key,
					set_id: *set_id,
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
			signature_data: SignatureData,
			event_id: ForeignEventId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let task_id = ObjectId(event_id.task_id().into());

			let schedule_data = T::TaskScheduleHelper::get_schedule_via_task_id(task_id)?;

			let Some(schedule) = schedule_data.first() else{
				return Err(Error::<T>::TaskNotScheduled.into());
			};

			let shard =
				<TssShards<T>>::get(schedule.shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;
			let collector = shard.collector();
			let collector_account_id = T::AccountId::decode(&mut collector.as_ref())
				.map_err(|_| Error::<T>::InvalidCollectorId)?;

			ensure!(caller == collector_account_id, Error::<T>::InvalidCaller);

			<SignatureStoreData<T>>::try_mutate(event_id, |signature_set| -> DispatchResult {
				ensure!(
					signature_set.get(&signature_data).is_none(),
					Error::<T>::DuplicateSignature
				);
				signature_set.insert(signature_data);
				Ok(())
			})?;

			Self::deposit_event(Event::SignatureStored(event_id));
			<LastCommittedChronicle<T>>::insert(
				collector,
				frame_system::Pallet::<T>::block_number(),
			);
			<LastCommittedShard<T>>::insert(
				schedule.shard_id,
				frame_system::Pallet::<T>::block_number(),
			);
			Ok(())
		}

		/// Submits TSS group key to runtime
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_tss_group_key(1))]
		pub fn submit_tss_group_key(
			origin: OriginFor<T>,
			set_id: u64,
			group_key: [u8; 33],
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			<TssGroupKey<T>>::insert(set_id, group_key);
			Self::deposit_event(Event::NewTssGroupKey(set_id, group_key));

			Ok(().into())
		}

		/// Root can register new shard via providing
		/// set of IDs matching one of supported size of shard
		/// # Param
		/// * members - supported sized set of shard members Id
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::register_shard())]
		pub fn register_shard(
			origin: OriginFor<T>,
			members: Vec<TimeId>,
			collector_index: Option<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let shard = new_shard::<T>(members.clone(), collector_index)?;
			// get unused ShardId from storage
			let shard_id = <ShardId<T>>::get();
			// compute next ShardId before putting it in storage
			let next_shard_id = shard_id.checked_add(1u64).ok_or(Error::<T>::ShardIdOverflow)?;
			<TssShards<T>>::insert(shard_id, shard);
			<ShardId<T>>::put(next_shard_id);
			Self::deposit_event(Event::ShardRegistered(shard_id));
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::register_chronicle())]
		pub fn register_chronicle(origin: OriginFor<T>, member: TimeId) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// get current validator set
			let validator_set = T::ValidatorSet::validators();

			// convert AccountId to ValidatorId
			let validator_id: <T::ValidatorSet as ValidatorSet<T::AccountId>>::ValidatorId =
				<T::ValidatorSet as ValidatorSet<T::AccountId>>::ValidatorIdOf::convert(
					caller.clone(),
				)
				.ok_or(Error::<T>::UnsupportedMembershipSize)?;

			// caller must be one of validators
			ensure!(validator_set.contains(&validator_id), Error::<T>::UnsupportedMembershipSize);

			// update chronicle worker set for caller
			ValidatorToChronicle::<T>::try_mutate(caller, |chronicles| match chronicles {
				Some(ref mut node) => {
					if node.contains(&member) {
						return Err::<(), Error<T>>(Error::<T>::UnsupportedMembershipSize.into());
					};

					node.try_insert(0, member).map_err(|_| Error::<T>::UnsupportedMembershipSize.into())?;
					Ok(())
				},
				None => {
					let _ = BoundedVec::<TimeId, T::MaxChronicleWorkers>::default()
						.try_insert(0, member);
					Ok(())
				},
			})?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn api_store_signature(
			auth_id: Public,
			auth_sig: Signature,
			signature_data: SignatureData,
			event_id: ForeignEventId,
		) -> DispatchResult {
			use sp_runtime::traits::AppVerify;
			// transform AccountId32 int T::AccountId
			let encoded_account = auth_id.encode();
			ensure!(encoded_account.len() == 32, Error::<T>::EncodedAccountWrongLen);
			ensure!(encoded_account[..] != [0u8; 32][..], Error::<T>::DefaultAccountForbidden);
			// TODO: same check as for extrinsic after task management is implemented
			// Update: implemnetation here is not necessary since this function will be removed
			// and only extrinsics will be used to store signature.
			ensure!(
				auth_sig.verify(signature_data.as_ref(), &auth_id),
				Error::<T>::UnregisteredWorkerDataSubmission
			);
			<SignatureStoreData<T>>::try_mutate(event_id, |signature_set| -> DispatchResult {
				ensure!(
					signature_set.get(&signature_data).is_none(),
					Error::<T>::DuplicateSignature
				);
				signature_set.insert(signature_data);
				Ok(())
			})?;
			Self::deposit_event(Event::SignatureStored(event_id));
			Ok(())
		}

		// Getter method for runtime api storage access
		pub fn api_tss_shards() -> Vec<(u64, Shard)> {
			<TssShards<T>>::iter().collect()
		}

		/// Method to provide misbehavior report to runtime
		/// Is protected with proven ownership of private key to prevent spam
		pub fn api_report_misbehavior(
			shard_id: u64,
			offender: time_primitives::TimeId,
			reporter: TimeId,
			proof: time_primitives::crypto::Signature,
		) -> DispatchResult {
			let reporter_pub =
				Public::from_slice(reporter.as_ref()).map_err(|_| Error::<T>::InvalidReporterId)?;
			let shard = <TssShards<T>>::get(shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;
			let members = shard.members();
			ensure!(
				members.contains(&offender) && members.contains(&reporter),
				Error::<T>::ReporterOrOffenderNotInMembers
			);
			// verify signature
			ensure!(
				proof.verify(offender.as_ref(), &reporter_pub),
				Error::<T>::ProofVerificationFailed
			);
			let reported_offences_count =
				if let Some(mut known_offender) = <ReportedOffences<T>>::get(&offender) {
					// do not allow more than one report per reporter
					ensure!(known_offender.1.insert(reporter), Error::<T>::MaxOneReportPerMember);
					// check reached threshold
					let shard_th = Percent::from_percent(T::SlashingPercentageThreshold::get())
						* members.len();
					let new_report_count = known_offender.0.saturating_plus_one();
					// update known offender report count
					known_offender.0 = new_report_count;
					if new_report_count.saturated_into::<usize>() >= shard_th {
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
					// do not allow more than one report per reporter
					ensure!(guilty_offender.1.insert(reporter), Error::<T>::MaxOneReportPerMember);
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
	}
}
