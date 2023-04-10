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
	use frame_support::{pallet_prelude::*, traits::Currency};
	use frame_system::pallet_prelude::*;
	use time_primitives::{ProxyAccInput, ProxyAccStatus, ProxyStatus};
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
	#[pallet::getter(fn get_proxy_status_store)]
	pub(super) type ProxyStorage<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		ProxyAccStatus<T::AccountId, BalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record account that uniquely identify
		ProxyStored(T::AccountId),

		///Already exist case
		ProxyAlreadyExist(T::AccountId),

		///Does not exist case
		ProxyNotExist(T::AccountId),

		///Proxy account suspended
		ProxySuspended(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The signing account has no permission to do the operation.
		NoPermission,
		/// Error getting schedule ref.
		ErrorRef,
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::weight(T::WeightInfo::store_schedule())]
		pub fn set_delegate_account(
			origin: OriginFor<T>,
			proxy: ProxyAccInput<BalanceOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let account_exit = self::ProxyStorage::<T>::iter_values()
				.find(|item| item.owner == who && item.status == ProxyStatus::Valid);
			// let account_exit = self::ProxyStorage::<T>::get(&who);
			match account_exit {
				Some(_acc) => {
					Self::deposit_event(Event::ProxyAlreadyExist(who));
				},
				None => {
					self::ProxyStorage::<T>::insert(
						who.clone(),
						ProxyAccStatus {
							owner: who.clone(),
							max_token_usage: proxy.max_token_usage,
							token_usage: proxy.token_usage,
							max_task_execution: proxy.max_task_execution,
							task_executed: proxy.task_executed,
							status: ProxyStatus::Valid,
						},
					);
					Self::deposit_event(Event::ProxyStored(who));
				},
			}

			Ok(())
		}
		#[pallet::weight(T::WeightInfo::store_schedule())]
		pub fn update_delegate_account(
			origin: OriginFor<T>,
			status: ProxyStatus,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let account_exit = self::ProxyStorage::<T>::iter_values()
				.find(|item| item.owner == who && item.status == ProxyStatus::Valid);
			match account_exit {
				Some(_acc) => {
					let _ = self::ProxyStorage::<T>::try_mutate(
						who.clone(),
						|proxy| -> DispatchResult {
							let details = proxy.as_mut().ok_or(Error::<T>::ErrorRef)?;
							ensure!(details.owner == who, Error::<T>::NoPermission);

							details.status = status;
							Ok(())
						},
					);
				},
				None => {
					Self::deposit_event(Event::ProxyNotExist(who));
				},
			}

			Ok(())
		}
		#[pallet::weight(T::WeightInfo::store_schedule())]
		pub fn remove_delegate_account(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let account_exit = self::ProxyStorage::<T>::iter_values()
				.find(|item| item.owner == who && item.status == ProxyStatus::Valid);
			match account_exit {
				Some(_acc) => {
					let _ = self::ProxyStorage::<T>::try_mutate(
						who.clone(),
						|proxy| -> DispatchResult {
							let details = proxy.as_mut().ok_or(Error::<T>::ErrorRef)?;
							ensure!(details.owner == who, Error::<T>::NoPermission);

							details.status = ProxyStatus::Suspended;
							Ok(())
						},
					);
					Self::deposit_event(Event::ProxySuspended(who));
				},
				None => {
					Self::deposit_event(Event::ProxyNotExist(who));
				},
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn _get_delegate_acc() -> Result<(), DispatchError> {
			// will add logic
			Ok(())
		}
		fn _charge_task_exe_fee() -> Result<(), DispatchError> {
			// will add logic

			Ok(())
		}

		fn _pay_task_fee() -> Result<(), DispatchError> {
			// will add logic

			Ok(())
		}
		fn _set_delegate_acc_invalid() -> Result<(), DispatchError> {
			// will add logic

			Ok(())
		}
	}
}
