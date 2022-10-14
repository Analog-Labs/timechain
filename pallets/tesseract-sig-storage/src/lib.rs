#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod types;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;

pub use pallet::*;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{ pallet_prelude::*, ensure};
	use frame_system::pallet_prelude::*;

	use crate::weights::WeightInfo;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
	}
	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn tesseract_members)]
	pub type TesseractMembers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, TesseractRole, OptionQuery>;


	#[pallet::storage]
	#[pallet::getter(fn signature_store)]
	pub type SignatureStore<T: Config> =
		StorageMap<_, Blake2_128Concat, SignatureKey, SignatureData, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the signature id that uniquely identify the signature
		SignatureStored(SignatureKey),

		/// A tesseract Node has been added as a member with it's role
		TesseractMemberAdded(T::AccountId, TesseractRole),

		/// A tesseract Node has been removed
		TesseractMemberRemoved(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The Tesseract address in not known
		UnknownTesseract
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::weight(T::WeightInfo::store_signature())]
		pub fn store_signature(
			origin: OriginFor<T>,
			signature_key: SignatureKey,
			signature_data: SignatureData,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ensure!(TesseractMembers::<T>::contains_key(caller),Error::<T>::UnknownTesseract);

			<SignatureStore<T>>::insert(signature_key.clone(), signature_data);

			Self::deposit_event(Event::SignatureStored(signature_key));

			Ok(())
		}

		/// Extrinsic for adding a node and it's member role
		/// Callable only by root for now
		#[pallet::weight(10000)]
		pub fn add_member(
			origin: OriginFor<T>,
			account: T::AccountId,
			role: TesseractRole,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			<TesseractMembers<T>>::insert(account.clone(), role.clone());

			Self::deposit_event(Event::TesseractMemberAdded(account, role));

			Ok(())
		}

		/// Extrinsic for adding a node and it's member role
		/// Callable only by root for now
		#[pallet::weight(10000)]
		pub fn remove_member(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			<TesseractMembers<T>>::remove(account.clone());

			Self::deposit_event(Event::TesseractMemberRemoved(account));

			Ok(())
		}
	}
}
