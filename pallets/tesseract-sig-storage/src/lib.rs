#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::Time};
	use frame_system::pallet_prelude::*;
	use scale_info::StaticTypeInfo;
	use sp_application_crypto::ByteArray;
	use sp_runtime::{
		traits::{AppVerify, Scale},
		Percent, SaturatedConversion,
	};
	use sp_std::{collections::btree_set::BTreeSet, result, vec::Vec};
	use time_primitives::{
		crypto::{Public, Signature},
		inherents::{InherentError, TimeTssKey, INHERENT_IDENTIFIER},
		sharding::Shard,
		ForeignEventId, SignatureData, TimeId,
	};

	pub trait WeightInfo {
		fn store_signature_data(_s: u32) -> Weight;
		fn submit_tss_group_key(s: u32) -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
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
	}

	/// Indicates precise members of each TSS set by it's u64 id
	/// Required for key generation and identification
	#[pallet::storage]
	#[pallet::getter(fn tss_shards)]
	pub type TssShards<T: Config> = StorageMap<_, Blake2_128Concat, u64, Shard, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tss_group_key)]
	pub type TssGroupKey<T: Config> = StorageMap<_, Blake2_128Concat, u64, [u8; 32], OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn signature_storage)]
	pub type SignatureStoreData<T: Config> =
		StorageMap<_, Blake2_128Concat, ForeignEventId, SignatureData, OptionQuery>;

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

		/// Unauthorized attempt to add signed data
		UnregisteredWorkerDataSubmission(T::AccountId),

		/// Default account is not allowed for this operation
		DefaultAccountForbidden(),

		/// New group key submitted to runtime
		/// .0 - set_id,
		/// .1 - group key bytes
		NewTssGroupKey(u64, [u8; 32]),

		/// Shard has ben registered with new Id
		ShardRegistered(u64),

		/// Offence report verification failed
		/// Identifies sender of faulty offence report
		ShardIsNotRegistered(TimeId),

		/// Reporter TimeId can not be converted to Public key
		WrongIdOfReporter(TimeId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The Tesseract address in not known
		UnknownTesseract,

		/// Shard registartion failed
		ShardRegistrationFailed,
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
					Some(inherent_data) if inherent_data.group_key != [0u8; 32] => {
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
		#[pallet::weight(T::WeightInfo::store_signature_data(1))]
		pub fn store_signature(
			origin: OriginFor<T>,
			signature_data: SignatureData,
			event_id: ForeignEventId,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;

			// TODO: based on 'event_id.task_id()' find task, get ShardId from it and check if
			// origin is a collector node of that shard this should be implemented after some task
			// management pallet is present and coupled with this one

			<SignatureStoreData<T>>::insert(event_id, signature_data);
			Self::deposit_event(Event::SignatureStored(event_id));
			Ok(())
		}

		/// Submits TSS group key to runtime
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_tss_group_key(1))]
		pub fn submit_tss_group_key(
			origin: OriginFor<T>,
			set_id: u64,
			group_key: [u8; 32],
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			<TssGroupKey<T>>::insert(set_id, group_key);
			Self::deposit_event(Event::NewTssGroupKey(set_id, group_key));

			Ok(().into())
		}

		/// Root can register new shard via providing not used set_id and
		/// set of IDs matching one of supported size of shard
		/// # Param
		/// * set_id - not yet used ID of new shard
		/// * members - supported sized set of shard members Id
		#[pallet::call_index(2)]
		#[pallet::weight(1_000_000)]
		pub fn register_shard(
			origin: OriginFor<T>,
			set_id: u64,
			members: Vec<TimeId>,
			collector: Option<TimeId>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(!<TssShards<T>>::contains_key(set_id), "Shard already registered.");
			if let Ok(mut shard) = Shard::try_from(members) {
				// we set specified collector if required
				if let Some(collector) = collector {
					ensure!(
						shard.set_collector(&collector).is_ok(),
						"Collector is not member of given set."
					);
				}
				<TssShards<T>>::insert(set_id, shard);
				Self::deposit_event(Event::ShardRegistered(set_id));
				Ok(())
			} else {
				Err(Error::<T>::ShardRegistrationFailed.into())
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn api_store_signature(
			auth_id: Public,
			auth_sig: Signature,
			signature_data: SignatureData,
			event_id: ForeignEventId,
		) {
			use sp_runtime::traits::AppVerify;
			// transform AccountId32 int T::AccountId
			let encoded_account = auth_id.encode();
			if encoded_account.len() != 32 || encoded_account == [0u8; 32].to_vec() {
				Self::deposit_event(Event::DefaultAccountForbidden());
				return;
			}
			// Unwrapping is safe - we've checked for len and default-ness
			let account_id = T::AccountId::decode(&mut &*encoded_account).unwrap();
			// TODO: same check as for extrinsic after task management is implemented
			if !auth_sig.verify(signature_data.as_ref(), &auth_id) {
				Self::deposit_event(Event::UnregisteredWorkerDataSubmission(account_id));
				return;
			}
			<SignatureStoreData<T>>::insert(event_id, signature_data);
			Self::deposit_event(Event::SignatureStored(event_id));
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
		) {
			if let Ok(reporter_pub) = Public::from_slice(reporter.as_ref()) {
				if let Some(shard) = <TssShards<T>>::get(shard_id) {
					let members = shard.members();
					// if reporter or offender are not from reported shard
					if !members.contains(&offender) || !members.contains(&reporter) {
						Self::deposit_event(Event::WrongIdOfReporter(reporter));
						return;
					}
					// verify signature
					if !proof.verify(offender.as_ref(), &reporter_pub) {
						return;
					}
					<ReportedOffence<T>>::mutate(&offender, |o| {
						if let Some(known_offender) = o {
							// check reached threshold
							let shard_th =
								Percent::from_percent(T::SlashingPercentageThreshold::get())
									* members.len();
							// move to commitment if reached
							if (known_offender.0 + 1).saturated_into::<usize>() >= shard_th {
								<CommitedOffences<T>>::insert(offender.clone(), known_offender);
							}
						// add if not
						} else {
							let mut hs = BTreeSet::new();
							hs.insert(reporter);
							// 1 here is count of reports received for this offence
							// incremented in above If section
							let _ = o.insert((1, hs));
						}
					});
				} else {
					Self::deposit_event(Event::ShardIsNotRegistered(reporter));
				}
			} else {
				Self::deposit_event(Event::WrongIdOfReporter(reporter));
			}
		}
	}
}
