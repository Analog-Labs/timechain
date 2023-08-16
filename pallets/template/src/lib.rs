#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Extrinsic_x(),
		Extrinsic_10x(),
		Extrinsic_x_db(),
		Extrinsic_10x_10db(),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::extrinsic_x())]
		pub fn extrinsic_x(origin: OriginFor<T>) -> DispatchResult {	
			let _who = ensure_signed(origin)?;	
			Self::deposit_event(Event::Extrinsic_x());	
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::extrinsic_10x())]
		pub fn extrinsic_10x(origin: OriginFor<T>) -> DispatchResult {	
			let _who = ensure_signed(origin)?;		
			Self::deposit_event(Event::Extrinsic_10x());	
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::extrinsic_x_db())]
		pub fn extrinsic_x_db(origin: OriginFor<T>) -> DispatchResult {	
			let _who = ensure_signed(origin)?;		
			Self::deposit_event(Event::Extrinsic_x_db());	
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::extrinsic_10x_10db())]
		pub fn extrinsic_10x_10db(origin: OriginFor<T>) -> DispatchResult {	
			let _who = ensure_signed(origin)?;	
			Self::deposit_event(Event::Extrinsic_10x_10db());		
			Ok(())
		}
	}
}
