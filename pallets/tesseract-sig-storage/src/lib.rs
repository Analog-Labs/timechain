#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	pub type ReferenceId = u128;
	pub type ThresholdSig = u128;
	pub type SigHash = u128;

	#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	pub struct Signature(ReferenceId, ThresholdSig);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn signatures)]
	pub type Signatures<T: Config> =
		StorageMap<_, Blake2_128Concat, SigHash, Signature, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature are :
		/// the extrinsic caller and the reference id
		SignatureStored(T::AccountId, ReferenceId),
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::weight(10_000)]
		pub fn store_signature(
			origin: OriginFor<T>,
			ref_id: ReferenceId,
			threshold_sig: ThresholdSig,
			signature_hash: SigHash,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			<Signatures<T>>::insert(signature_hash, Signature(ref_id, threshold_sig));

			Self::deposit_event(Event::SignatureStored(caller, ref_id));

			Ok(())
		}
	}
}
