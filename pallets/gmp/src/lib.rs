#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

// include information for
// (1) registering the shard
// (2) forming the submit (RegisterShard) call and calling it

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{IdentifyAccount, Saturating};
	use sp_std::vec;
	use time_primitives::{AccountId, TssSignature};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MessageSent(),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn send_message(
			origin: OriginFor<T>,
			_message: vec::Vec<u8>,
			_inputs: TssSignature,
		) -> DispatchResult {
			ensure_signed(origin)?;
			// inherit pallet_evm through a trait or directly
			// TODO: eth.call_smart_contract()
			Self::deposit_event(Event::MessageSent());
			Ok(())
		}
	}
}
