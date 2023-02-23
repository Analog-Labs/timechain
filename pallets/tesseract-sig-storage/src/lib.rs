#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
mod types;

pub mod weights;
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use crate::types::*;
	use frame_support::{pallet_prelude::*, sp_runtime::traits::Scale, traits::Time};
	use frame_system::pallet_prelude::*;
	use scale_info::StaticTypeInfo;
	use sp_std::{result, vec::Vec};
	use time_primitives::{
		crypto::{Public, Signature},
		inherents::{InherentError, TimeTssKey, INHERENT_IDENTIFIER},
		sharding::Shard,
		ForeignEventId, SignatureData, TimeId,
	};

	pub trait WeightInfo {
		fn add_member() -> Weight;
		fn store_signature_data() -> Weight;
		fn remove_member() -> Weight;
		fn submit_tss_group_key() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
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
	}

	#[pallet::storage]
	#[pallet::getter(fn tesseract_members)]
	pub type TesseractMembers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, TesseractRole, OptionQuery>;

	/// Indicates precise members of each TSS set by it's u64 id
	/// Required for key generation and identification
	#[pallet::storage]
	#[pallet::getter(fn tss_set)]
	pub type TssShards<T: Config> = StorageMap<_, Blake2_128Concat, u64, Shard, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tss_group_key)]
	pub type TssGroupKey<T: Config> = StorageMap<_, Blake2_128Concat, u64, [u8; 32], OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn signature_storage)]
	pub type SignatureStoreData<T: Config> =
		StorageMap<_, Blake2_128Concat, ForeignEventId, SignatureStorage<T::Moment>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the signature id that uniquely identify the signature
		SignatureStored(ForeignEventId),

		/// A tesseract Node has been added as a member with it's role
		TesseractMemberAdded(T::AccountId, TesseractRole),

		/// A tesseract Node has been removed
		TesseractMemberRemoved(T::AccountId),

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
		#[pallet::weight(T::WeightInfo::store_signature_data())]
		pub fn store_signature(
			origin: OriginFor<T>,
			signature_data: SignatureData,
			event_id: ForeignEventId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(TesseractMembers::<T>::contains_key(caller), Error::<T>::UnknownTesseract);
			let storage_data = SignatureStorage::new(signature_data.clone(), T::Timestamp::now());

			<SignatureStoreData<T>>::insert(event_id, storage_data);

			Self::deposit_event(Event::SignatureStored(event_id));

			Ok(())
		}

		/// Extrinsic for adding a node and it's member role
		/// Callable only by root for now
		#[pallet::weight(T::WeightInfo::add_member())]
		pub fn add_member(
			origin: OriginFor<T>,
			account: T::AccountId,
			role: TesseractRole,
		) -> DispatchResult {
			let _ = ensure_signed_or_root(origin)?;

			<TesseractMembers<T>>::insert(account.clone(), role.clone());

			Self::deposit_event(Event::TesseractMemberAdded(account, role));

			Ok(())
		}

		/// Extrinsic for adding a node and it's member role
		/// Callable only by root for now
		#[pallet::weight(T::WeightInfo::remove_member())]
		pub fn remove_member(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			let _ = ensure_signed_or_root(origin)?;

			<TesseractMembers<T>>::remove(account.clone());

			Self::deposit_event(Event::TesseractMemberRemoved(account));

			Ok(())
		}

		/// Submits TSS group key to runtime
		#[pallet::weight(T::WeightInfo::submit_tss_group_key())]
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
				Ok(().into())
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
			if !TesseractMembers::<T>::contains_key(account_id.clone())
				|| !auth_sig.verify(&*signature_data, &auth_id)
			{
				Self::deposit_event(Event::UnregisteredWorkerDataSubmission(account_id));
				return;
			}
			let storage_data = SignatureStorage::new(signature_data, T::Timestamp::now());

			<SignatureStoreData<T>>::insert(event_id, storage_data);

			Self::deposit_event(Event::SignatureStored(event_id));
		}
	}
}
