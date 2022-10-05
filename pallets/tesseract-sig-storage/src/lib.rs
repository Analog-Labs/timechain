#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::inherent::Vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use crate::weights::WeightInfo;

	/// type that uniquely identify a signature data
	pub type SignatureKey = Vec<u8>;

	/// The type representing a signature data
	pub type SignatureData = Vec<u8>;
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
	#[pallet::getter(fn signature_store)]
	pub type SignatureStore<T: Config> =
		StorageMap<_, Blake2_128Concat, SignatureKey, SignatureData, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the signature id that uniquely identify the signature
		SignatureStored(SignatureKey),
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::weight(T::WeightInfo::store_signature())]
		pub fn store_signature(
			origin: OriginFor<T>,
			signature_key: SignatureKey,
			signature_data: SignatureData,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			<SignatureStore<T>>::insert(signature_key.clone(), signature_data);

			Self::deposit_event(Event::SignatureStored(signature_key));

			Ok(())
		}
	}
}
