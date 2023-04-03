#![cfg_attr(not(feature = "std"), no_std)]
pub mod weights;

use sp_std::prelude::*;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::traits::Currency;
	use log::info;
	use time_primitives::DelegationStatus;
	pub type KeyId = u64;
	pub(crate) type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	
	pub trait WeightInfo {
		fn store_schedule() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Currency: Currency<Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_task_schedule)]
	pub(super) type ScheduleStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, DelegationStatus<T::AccountId, BalanceOf<T>>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the record id that uniquely identify
		ScheduleStored(KeyId),

		///Already exist case
		AlreadyExist(KeyId),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::weight(T::WeightInfo::store_schedule())]
		pub fn set_delegate_account(origin: OriginFor<T>) -> DispatchResult {

			Ok(())
		}
		#[pallet::weight(T::WeightInfo::store_schedule())]
		pub fn update_delegate_account(origin: OriginFor<T>) -> DispatchResult {

			Ok(())
		}
		#[pallet::weight(T::WeightInfo::store_schedule())]
		pub fn remove_delegate_account(origin: OriginFor<T>) -> DispatchResult {

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_delegate_acc() -> Result<(), DispatchError> {
			// will add scheduling logic

			Ok(())
		}
		pub fn charge_task_exe_fee() -> Result<(), DispatchError> {
			// will add scheduling logic

			Ok(())
		}

		fn pay_task_fee() -> Result<(), DispatchError> {
			// will add scheduling logic

			Ok(())
		}
		fn set_delegate_acc_invalid() -> Result<(), DispatchError> {
			// will add scheduling logic

			Ok(())
		}
	}
}