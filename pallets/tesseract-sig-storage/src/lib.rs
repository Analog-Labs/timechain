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
	use crate::{types::*, weights::WeightInfo};
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::traits::{Hash, Scale},
		traits::{Randomness, Time},
	};
	use frame_system::pallet_prelude::*;
	use scale_info::StaticTypeInfo;
	use sp_runtime::{app_crypto::RuntimePublic, traits::IdentifyAccount};
	use sp_std::vec::Vec;
	use time_primitives::{SignatureData, TimeKey, TimeSignature};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type StoreRandomness: Randomness<Self::Hash, Self::BlockNumber>;
		type Moment: Parameter
			+ Default
			+ Scale<Self::BlockNumber, Output = Self::Moment>
			+ Copy
			+ MaxEncodedLen
			+ StaticTypeInfo;
		type Timestamp: Time<Moment = Self::Moment>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_nonce)]
	pub(super) type Nonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tesseract_members)]
	pub type TesseractMembers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, TesseractRole, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn signature_store)]
	pub type SignatureStore<T: Config> =
		StorageMap<_, Blake2_128Concat, T::Hash, SignatureData, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn signature_storage)]
	pub type SignatureStoreData<T: Config> =
		StorageMap<_, Blake2_128Concat, T::Hash, SignatureStorage<T::Hash, T::Moment>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the signature id that uniquely identify the signature
		SignatureStored(T::Hash, SignatureData),

		/// A tesseract Node has been added as a member with it's role
		TesseractMemberAdded(T::AccountId, TesseractRole),

		/// A tesseract Node has been removed
		TesseractMemberRemoved(T::AccountId),

		/// Unauthorized attemtp to add signed data
		UnregisteredWorkerDataSubmission(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The Tesseract address in not known
		UnknownTesseract,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::weight(T::WeightInfo::store_signature_data())]
		pub fn store_signature(
			origin: OriginFor<T>,
			signature_data: SignatureData,
			network_id: Vec<u8>,
			block_height: u64,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(
				TesseractMembers::<T>::contains_key(caller.clone()),
				Error::<T>::UnknownTesseract
			);
			let random_value = Self::random_hash(&caller);
			let storage_data = SignatureStorage::new(
				random_value.clone(),
				signature_data.clone(),
				network_id.to_vec().clone(),
				block_height,
				T::Timestamp::now(),
			);

			<SignatureStoreData<T>>::insert(random_value.clone(), storage_data);

			Self::deposit_event(Event::SignatureStored(random_value, signature_data));

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
	}

	impl<T: Config> Pallet<T> {
		fn random_hash(sender: &T::AccountId) -> T::Hash {
			let nonce = <Nonce<T>>::get();
			let seed = T::StoreRandomness::random_seed();

			T::Hashing::hash_of(&(seed, sender, nonce))
		}

		pub fn api_store_signature(
			auth_id: TimeKey,
			auth_sig: TimeSignature,
			signature_data: SignatureData,
			network_id: Vec<u8>,
			block_height: u64,
		) {
			if !TesseractMembers::<T>::contains_key(auth_id.clone()) ||
				!auth_id.verify(&signature_data, &auth_sig)
			{
				Self::deposit_event(Event::UnregisteredWorkerDataSubmission(auth_id.into_account()))
			}
			let random_value = Self::random_hash(&auth_id);
			let storage_data = SignatureStorage::new(
				random_value.clone(),
				signature_data.clone(),
				network_id.to_vec().clone(),
				block_height,
				T::Timestamp::now(),
			);

			<SignatureStoreData<T>>::insert(random_value.clone(), storage_data);

			Self::deposit_event(Event::SignatureStored(random_value, signature_data));
		}
	}
}
