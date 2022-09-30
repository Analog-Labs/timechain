#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::inherent::Vec;
	

	pub type SignatureData = Vec<u8>;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);



	#[pallet::storage]
	#[pallet::getter(fn signature_data)]

	pub type SignatureDataStore<T> = StorageValue<_, SignatureData>;


	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature are :
		/// the extrinsic caller and the reference id
		SignatureStored(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::weight(10_000)]
		pub fn store_signature(
			origin: OriginFor<T>,
			signature_data: SignatureData,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			<SignatureDataStore<T>>::put(signature_data);

			Self::deposit_event(Event::SignatureStored(caller));

			Ok(())
		}
	}
}
