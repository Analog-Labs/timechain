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
	// use frame_system::offchain::SendSignedTransaction;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::vec::Vec;
	// use sp_core::crypto::KeyTypeId;
	use time_primitives::{
		abstraction::{
			ObjectId, PayableScheduleInput, PayableTaskSchedule, ScheduleInput, ScheduleStatus,
			TaskSchedule,
		},
		PalletAccounts, ProxyExtend,
	};

	pub type KeyId = u64;
	pub type ScheduleResults<AccountId> = Vec<(KeyId, TaskSchedule<AccountId>)>;
	pub type PayableScheduleResults<AccountId> = Vec<(KeyId, PayableTaskSchedule<AccountId>)>;
	pub trait WeightInfo {
		fn insert_schedule() -> Weight;
		fn update_schedule() -> Weight;
		fn insert_payable_schedule() -> Weight;
		fn update_execution_state() -> Weight;
	}

	// pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"demo");

	// pub mod crypto {
	// 	use super::KEY_TYPE;
	// 	use sp_core::sr25519::Signature as Sr25519Signature;
	// 	use sp_runtime::{
	// 		app_crypto::{app_crypto, sr25519},
	// 		traits::Verify,
	// 		MultiSignature, MultiSigner,
	// 	};
	// 	app_crypto!(sr25519, KEY_TYPE);

	// 	pub struct TestAuthId;

	// 	// implemented for runtime
	// 	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
	// 		type RuntimeAppPublic = Public;
	// 		type GenericSignature = sp_core::sr25519::Signature;
	// 		type GenericPublic = sp_core::sr25519::Public;
	// 	}
	// }

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type ProxyExtend: ProxyExtend<Self::AccountId>;
		type Currency: Currency<Self::AccountId>;
		type PalletAccounts: PalletAccounts<Self::AccountId>;
		type ScheduleFee: Get<u32>;
		type ExecutionFee: Get<u32>;
		// type AuthorityId: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_task_schedule)]
	pub type ScheduleStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, KeyId, TaskSchedule<T::AccountId>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_payable_task_schedule)]
	pub type PayableScheduleStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, KeyId, PayableTaskSchedule<T::AccountId>, OptionQuery>;

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

		// Payable Schedule
		PayableScheduleStored(KeyId),

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
					frequency: schedule.frequency,
					validity: schedule.validity,
					hash: schedule.hash,
					start_execution_block: 0,
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
		#[pallet::weight(T::WeightInfo::insert_payable_schedule())]
		pub fn insert_payable_task_schedule(
			origin: OriginFor<T>,
			schedule: PayableScheduleInput,
		) -> DispatchResult {
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
			self::PayableScheduleStorage::<T>::insert(
				schedule_id,
				PayableTaskSchedule {
					task_id: schedule.task_id,
					owner: who,
					shard_id: schedule.shard_id,
					status: ScheduleStatus::Initiated,
				},
			);
			Self::deposit_event(Event::PayableScheduleStored(schedule_id));

			Ok(())
		}

		#[pallet::call_index(3)]
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
		// pub fn offchain_worker(_block_number: T::BlockNumber) {
		// 	let signer = frame_system::offchain::Signer::<T, T::AuthorityId>::all_accounts();

		// 	let results = signer.send_signed_transaction(|_account| Call::update_schedule {
		// 		status: ScheduleStatus::Completed,
		// 		key: 1,
		// 	});

		// 	// ...

		// 	for (acc, res) in &results {
		// 		match res {
		// 			Ok(()) => {
		// 				frame_support::log::info!("[{:?}]: submit transaction success.", acc.id)
		// 			},
		// 			Err(e) => frame_support::log::error!(
		// 				"[{:?}]: submit transaction failure. Reason: {:?}",
		// 				acc.id,
		// 				e
		// 			),
		// 		}
		// 	}
		// }

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

		pub fn update_failed_schedule_by_key(
			status: ScheduleStatus,
			key: KeyId,
		) -> Result<(), DispatchError> {
			let _ = self::ScheduleStorage::<T>::try_mutate(key, |schedule| -> DispatchResult {
				let details = schedule.as_mut().ok_or(Error::<T>::ErrorRef)?;
				details.status = status.clone();
				if status == ScheduleStatus::Recurring {
					details.cycle += 1;
				}
				Ok(())
			});

			Ok(())
		}

		pub fn get_payable_task_schedules(
		) -> Result<PayableScheduleResults<T::AccountId>, DispatchError> {
			let data_list = self::PayableScheduleStorage::<T>::iter()
				.filter(|item| item.1.status == ScheduleStatus::Initiated)
				.collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_payable_schedules_by_task_id(
			key: ObjectId,
		) -> Result<Vec<PayableTaskSchedule<T::AccountId>>, DispatchError> {
			let data = self::PayableScheduleStorage::<T>::iter_values()
				.filter(|item| item.task_id == key)
				.collect::<Vec<_>>();

			Ok(data)
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

	// #[pallet::hooks]
	// impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
	// 	/// Offchain worker entry point.
	// 	fn offchain_worker(block_number: T::BlockNumber) {
	// 		frame_support::log::info!("Hello from pallet-ocw.{:?}", block_number);
	// 		// The entry point of your code called by offchain worker
	// 	}
	// }
}
