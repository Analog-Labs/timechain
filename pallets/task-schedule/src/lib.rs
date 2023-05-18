#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement::KeepAlive},
	};

	use frame_system::pallet_prelude::*;
	use scale_info::prelude::vec::Vec;
	use time_primitives::{
		abstraction::{ObjectId, ScheduleInput, ScheduleStatus, TaskSchedule},
		PalletAccounts, ProxyExtend,
	};
	pub type KeyId = u64;
	pub type ScheduleResults<AccountId> = Vec<(KeyId, TaskSchedule<AccountId>)>;
	pub trait WeightInfo {
		fn insert_schedule() -> Weight;
		fn update_schedule() -> Weight;
		fn update_execution_state() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type ProxyExtend: ProxyExtend<Self::AccountId>;
		type Currency: Currency<Self::AccountId>;
		type PalletAccounts: PalletAccounts<Self::AccountId>;
		type ScheduleFee: Get<u32>;
		type ExecutionFee: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_task_schedule)]
	pub type ScheduleStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, KeyId, TaskSchedule<T::AccountId>, OptionQuery>;

	#[pallet::storage]
	pub(super) type LastKey<T: Config> = StorageValue<_, u64, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		ScheduleStored(KeyId),

		/// Updated Schedule
		ScheduleUpdated(KeyId),

		///Already exist case
		AlreadyExist(KeyId),

		///Execution Status Updated
		ExecutionStatusUpdated(KeyId),

		///Schedule Not Found
		ScheduleNotFound(KeyId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The signing account has no permission to do the operation.
		NoPermission,
		/// Not a valid submitter
		NotProxyAccount,
		/// Fee couldn't deducted
		FeeDeductIssue,
		/// Proxy account(s) token usage not updated
		ProxyNotUpdated,
		/// Error getting schedule ref.
		ErrorRef,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::insert_schedule())]
		pub fn insert_schedule(origin: OriginFor<T>, schedule: ScheduleInput) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let fix_fee = T::ScheduleFee::get();
			let resp = T::ProxyExtend::proxy_exist(who.clone());
			ensure!(resp, Error::<T>::NotProxyAccount);
			let treasury = T::PalletAccounts::get_treasury();

			let tokens_updated = T::ProxyExtend::proxy_update_token_used(who.clone(), fix_fee);
			ensure!(tokens_updated, Error::<T>::ProxyNotUpdated);
			let master_acc = T::ProxyExtend::get_master_account(who.clone()).unwrap();
			let res = T::Currency::transfer(&master_acc, &treasury, fix_fee.into(), KeepAlive);

			ensure!(res.is_ok(), Error::<T>::FeeDeductIssue);

			let last_key = self::LastKey::<T>::get();
			let schedule_id = match last_key {
				Some(val) => val.saturating_add(1),
				None => 1,
			};
			self::LastKey::<T>::put(schedule_id);
			self::ScheduleStorage::<T>::insert(
				schedule_id,
				TaskSchedule {
					task_id: schedule.task_id,
					owner: who,
					shard_id: schedule.shard_id,
					cycle: schedule.cycle,
					validity: schedule.validity,
					hash: schedule.hash,
					status: ScheduleStatus::Initiated,
				},
			);
			Self::deposit_event(Event::ScheduleStored(schedule_id));

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_schedule())]
		pub fn update_schedule(
			origin: OriginFor<T>,
			status: ScheduleStatus,
			key: KeyId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let resp = T::ProxyExtend::proxy_exist(who.clone());
			ensure!(resp, Error::<T>::NotProxyAccount);
			let _ = self::ScheduleStorage::<T>::try_mutate(key, |schedule| -> DispatchResult {
				let details = schedule.as_mut().ok_or(Error::<T>::ErrorRef)?;
				ensure!(details.owner == who, Error::<T>::NoPermission);

				details.status = status.clone();
				if status == ScheduleStatus::Recurring && details.cycle > 0 {
					details.cycle -= 1;
				}

				Ok(())
			});
			Self::deposit_event(Event::ScheduleUpdated(key));

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::update_execution_state())]
		pub fn update_execution_state(origin: OriginFor<T>, schedule_id: u64) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let fix_fee = T::ExecutionFee::get();
			let treasury = T::PalletAccounts::get_treasury();
			let schedule_info = self::ScheduleStorage::<T>::get(schedule_id);
			match schedule_info {
				Some(schedule) => {
					let master_acc = schedule.owner;
					let res =
						T::Currency::transfer(&master_acc, &treasury, fix_fee.into(), KeepAlive);
					ensure!(res.is_ok(), Error::<T>::FeeDeductIssue);
					Self::deposit_event(Event::ExecutionStatusUpdated(schedule_id));
				},
				None => {
					Self::deposit_event(Event::ScheduleNotFound(schedule_id));
				},
			}

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn get_schedules() -> Result<ScheduleResults<T::AccountId>, DispatchError> {
			let data_list = self::ScheduleStorage::<T>::iter()
				.filter(|item| {
					item.1.status == ScheduleStatus::Initiated
						|| (item.1.status == ScheduleStatus::Recurring && item.1.cycle > 0)
				})
				.collect::<Vec<_>>();

			Ok(data_list)
		}
		pub fn get_schedules_by_task_id(
			key: ObjectId,
		) -> Result<Vec<TaskSchedule<T::AccountId>>, DispatchError> {
			let data = self::ScheduleStorage::<T>::iter_values()
				.filter(|item| item.task_id == key)
				.collect::<Vec<_>>();

			Ok(data)
		}
		pub fn get_schedules_keys() -> Result<Vec<u64>, DispatchError> {
			let data_list = self::ScheduleStorage::<T>::iter_keys().collect::<Vec<_>>();

			Ok(data_list)
		}
		pub fn get_schedule_by_key(
			key: u64,
		) -> Result<Option<TaskSchedule<T::AccountId>>, DispatchError> {
			let data = self::ScheduleStorage::<T>::get(key);

			Ok(data)
		}

		pub fn update_schedule_by_key(
			status: ScheduleStatus,
			key: KeyId,
		) -> Result<(), DispatchError> {
			let _ = self::ScheduleStorage::<T>::try_mutate(key, |schedule| -> DispatchResult {
				let details = schedule.as_mut().ok_or(Error::<T>::ErrorRef)?;
				details.status = status.clone();
				if status == ScheduleStatus::Recurring && details.cycle > 0 {
					details.cycle -= 1;
				}
				Ok(())
			});

			Ok(())
		}

		pub fn update_execution(schedule_id: u64) -> Result<(), DispatchError> {
			let fix_fee = T::ExecutionFee::get();
			let treasury = T::PalletAccounts::get_treasury();
			let schedule_info = self::ScheduleStorage::<T>::get(schedule_id);
			match schedule_info {
				Some(schedule) => {
					let master_acc = schedule.owner;
					let res =
						T::Currency::transfer(&master_acc, &treasury, fix_fee.into(), KeepAlive);
					ensure!(res.is_ok(), Error::<T>::FeeDeductIssue);
					Ok(())
				},
				None => Ok(()),
			}
		}
	}
}
