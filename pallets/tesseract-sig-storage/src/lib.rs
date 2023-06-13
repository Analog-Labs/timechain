#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::crypto::KeyTypeId;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"psig");

pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);
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
		traits::Time,
	};
	use frame_system::offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::StaticTypeInfo;
	use sp_application_crypto::ByteArray;
	use sp_runtime::offchain::storage::StorageValueRef;
	use sp_runtime::{
		traits::{AppVerify, Scale},
		Percent, SaturatedConversion, Saturating,
	};
	use sp_std::{
		collections::{btree_set::BTreeSet, vec_deque::VecDeque},
		result,
		vec::Vec,
	};
	use task_schedule::ScheduleFetchInterface;
	use time_primitives::{
		abstraction::{OCWSigData, ObjectId},
		crypto::{Public, Signature},
		inherents::{InherentError, TimeTssKey, INHERENT_IDENTIFIER},
		sharding::Shard,
		ForeignEventId, SignatureData, TimeId, OCW_SIG_KEY,
	};

	pub trait WeightInfo {
		fn store_signature(_s: u32) -> Weight;
		fn submit_tss_group_key(_s: u32) -> Weight;
		fn register_shard() -> Weight;
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
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(_block_number: T::BlockNumber) {
			let storage_ref = StorageValueRef::persistent(OCW_SIG_KEY);
			let res = storage_ref.get::<VecDeque<OCWSigData>>();
			let Ok(data) = res else {
				log::error!("could not fetch data from storage");
				return;
			};

			let Some(mut sig_vec) = data else {
				return;
			};

			let Some(sig_req) = sig_vec.pop_front()else{
				log::info!("no signature request");
				return;
			};

			if let Err(err) = Self::ocw_store_signature(sig_req) {
				log::error!("Error occured while submitting extrinsic {:?}", err);
			};

			storage_ref.set(&sig_vec);
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

		type TaskScheduleHelper: ScheduleFetchInterface<Self::AccountId>;
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

		/// Invalid validation signature
		InvalidValidationSignature,

		///TSS Signature already added
		DuplicateSignature,

		///Offchain signed tx failed
		OffchainSignedTxFailed,

		///no local account for signed tx
		NoLocalAcctForSignedTx,
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
			auth_sig: Signature,
			signature_data: SignatureData,
			event_id: ForeignEventId,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let task_id = ObjectId(event_id.task_id().into());

			let schedule_data = T::TaskScheduleHelper::get_schedule_via_task_id(task_id)?;
			let payable_schedule_data =
				T::TaskScheduleHelper::get_payable_schedules_via_task_id(task_id)?;

			let shard_id = if let Some(schedule) = schedule_data.first() {
				schedule.shard_id
			} else if let Some(payable_schedule) = payable_schedule_data.first() {
				payable_schedule.shard_id
			} else {
				return Err(Error::<T>::TaskNotScheduled.into());
			};

			let shard = <TssShards<T>>::get(shard_id).ok_or(Error::<T>::ShardIsNotRegistered)?;
			let collector = shard.collector();

			let raw_public_key: &[u8; 32] = collector.as_ref();
			let collector_public_id =
				sp_application_crypto::sr25519::Public::from_raw(*raw_public_key);

			ensure!(
				auth_sig.verify(signature_data.as_ref(), &collector_public_id.into()),
				Error::<T>::InvalidValidationSignature
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
			let shard = new_shard::<T>(members, collector_index)?;
			// get unused ShardId from storage
			let shard_id = <ShardId<T>>::get();
			// compute next ShardId before putting it in storage
			let next_shard_id = shard_id.checked_add(1u64).ok_or(Error::<T>::ShardIdOverflow)?;
			<TssShards<T>>::insert(shard_id, shard);
			<ShardId<T>>::put(next_shard_id);
			Self::deposit_event(Event::ShardRegistered(shard_id));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
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

		fn ocw_store_signature(data: OCWSigData) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();

			if let Some((acc, res)) =
				signer.send_signed_transaction(|_account| Call::store_signature {
					auth_sig: data.auth_sig.clone(),
					signature_data: data.sig_data,
					event_id: data.event_id,
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
