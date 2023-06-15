#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub mod crypto {
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	use time_primitives::SKD_KEY_TYPE;
	app_crypto!(sr25519, SKD_KEY_TYPE);
	pub struct SigAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for SigAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for SigAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement::KeepAlive},
	};
	use log;
	use sp_runtime::traits::Saturating;

	use frame_system::offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
	};

	use frame_system::pallet_prelude::*;
	use pallet_session::ShouldEndSession;
	use scale_info::prelude::vec::Vec;

	use sp_runtime::offchain::storage::{
		MutateStorageError, StorageRetrievalError, StorageValueRef,
	};
	use sp_std::collections::vec_deque::VecDeque;
	use time_primitives::{
		abstraction::{
			OCWSkdData, ObjectId, PayableScheduleInput, PayableTaskSchedule, ScheduleInput,
			ScheduleStatus, TaskSchedule,
		},
		PalletAccounts, ProxyExtend, OCW_SKD_KEY,
	};
	pub(crate) type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type KeyId = u64;
	pub type ScheduleResults<AccountId> = Vec<(KeyId, TaskSchedule<AccountId>)>;
	pub type PayableScheduleResults<AccountId> = Vec<(KeyId, PayableTaskSchedule<AccountId>)>;
	pub trait WeightInfo {
		fn insert_schedule() -> Weight;
		fn update_schedule() -> Weight;
		fn insert_payable_schedule() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(_block_number: T::BlockNumber) {
			let storage_ref = StorageValueRef::persistent(OCW_SKD_KEY);

			const EMPTY_DATA: () = ();

			let outer_res = storage_ref.mutate(
				|res: Result<Option<VecDeque<OCWSkdData>>, StorageRetrievalError>| {
					match res {
						Ok(Some(mut data)) => {
							// iteration batch of 5
							for _ in 0..5 {
								if let Some(skd_req) = data.pop_front() {
									if let Err(err) =
										Self::ocw_update_schedule_by_key(skd_req.clone())
									{
										log::error!(
											"Error occured while submitting extrinsic {:?}",
											err
										);
									};
								} else {
									break;
								}
							}
							Ok(data)
						},
						Ok(None) => Err(EMPTY_DATA),
						Err(_) => Err(EMPTY_DATA),
					}
				},
			);

			match outer_res {
				Err(MutateStorageError::ValueFunctionFailed(EMPTY_DATA)) => {
					log::info!("Task schedule OCW is empty");
				},
				Err(MutateStorageError::ConcurrentModification(_)) => {
					log::error!("💔 Error updating local storage in SKD OCW",);
				},
				Ok(_) => {},
			}
		}

		fn on_initialize(now: T::BlockNumber) -> Weight {
			if T::ShouldEndSession::should_end_session(now) {
				for (indexer, times) in IndexerScore::<T>::iter() {
					let reward_amount = T::IndexerReward::get().saturating_mul(times.into());
					let result = T::Currency::deposit_into_existing(&indexer, reward_amount);
					if result.is_err() {
						log::error!(
							"Failed to reward account {:?} with {:?}.",
							&indexer,
							reward_amount
						);
					} else {
						Self::deposit_event(Event::RewardIndexer(indexer, reward_amount));
					}
				}

				T::BlockWeights::get().max_block
			} else {
				Weight::zero()
			}
		}
	}

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type ProxyExtend: ProxyExtend<Self::AccountId, BalanceOf<Self>>;
		type Currency: Currency<Self::AccountId>;
		type PalletAccounts: PalletAccounts<Self::AccountId>;
		type ScheduleFee: Get<BalanceOf<Self>>;
		type ShouldEndSession: ShouldEndSession<Self::BlockNumber>;
		type IndexerReward: Get<BalanceOf<Self>>;
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

	#[pallet::storage]
	#[pallet::getter(fn indexer_score)]
	pub type IndexerScore<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		ScheduleStored(KeyId),

		/// Updated Schedule
		ScheduleUpdated(KeyId),

		///Already exist case
		AlreadyExist(KeyId),

		/// Payable Schedule
		PayableScheduleStored(KeyId),

		/// Reward indexer
		RewardIndexer(T::AccountId, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The signing account has no permission to do the operation.
		NoPermission,
		/// Not a valid submitter
		NotProxyAccount,
		/// Proxy account(s) token usage not updated
		ProxyNotUpdated,
		/// Error getting schedule ref.
		ErrorRef,
		/// Failed to reward indexer
		FailedRewardIndexer,
		///Offchain signed tx failed
		OffchainSignedTxFailed,
		///no local account for signed tx
		NoLocalAcctForSignedTx,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::insert_schedule())]
		pub fn insert_schedule(origin: OriginFor<T>, schedule: ScheduleInput) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let fix_fee = T::ScheduleFee::get();
			let resp = T::ProxyExtend::proxy_exist(&who);
			ensure!(resp, Error::<T>::NotProxyAccount);
			let treasury = T::PalletAccounts::get_treasury();

			let tokens_updated = T::ProxyExtend::proxy_update_token_used(&who, fix_fee);
			ensure!(tokens_updated, Error::<T>::ProxyNotUpdated);
			let master_acc = T::ProxyExtend::get_master_account(&who).unwrap();
			T::Currency::transfer(&master_acc, &treasury, fix_fee, KeepAlive)?;

			let last_key = LastKey::<T>::get();
			let schedule_id = match last_key {
				Some(val) => val.saturating_add(1),
				None => 1,
			};
			LastKey::<T>::put(schedule_id);
			ScheduleStorage::<T>::insert(
				schedule_id,
				TaskSchedule {
					task_id: schedule.task_id,
					owner: who,
					shard_id: schedule.shard_id,
					cycle: schedule.cycle,
					start_block: 0,
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
			let resp = T::ProxyExtend::proxy_exist(&who);
			ensure!(resp, Error::<T>::NotProxyAccount);
			let _ = ScheduleStorage::<T>::try_mutate(key, |schedule| -> DispatchResult {
				let details = schedule.as_mut().ok_or(Error::<T>::ErrorRef)?;
				ensure!(details.owner == who, Error::<T>::NoPermission);

				details.status = status;
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
			let resp = T::ProxyExtend::proxy_exist(&who);
			ensure!(resp, Error::<T>::NotProxyAccount);
			let treasury = T::PalletAccounts::get_treasury();

			let tokens_updated = T::ProxyExtend::proxy_update_token_used(&who, fix_fee);
			ensure!(tokens_updated, Error::<T>::ProxyNotUpdated);
			let master_acc = T::ProxyExtend::get_master_account(&who).unwrap();
			T::Currency::transfer(&master_acc, &treasury, fix_fee, KeepAlive)?;

			let last_key = LastKey::<T>::get();
			let schedule_id = match last_key {
				Some(val) => val.saturating_add(1),
				None => 1,
			};
			LastKey::<T>::put(schedule_id);
			PayableScheduleStorage::<T>::insert(
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
	}
	impl<T: Config> Pallet<T> {
		pub fn update_indexer(indexer: T::AccountId) -> Result<(), DispatchError> {
			IndexerScore::<T>::mutate(indexer, |reward| *reward += 1);
			Ok(())
		}

		pub fn get_schedules() -> Result<ScheduleResults<T::AccountId>, DispatchError> {
			let data_list = ScheduleStorage::<T>::iter()
				.filter(|item| item.1.status == ScheduleStatus::Initiated)
				.collect::<Vec<_>>();

			Ok(data_list)
		}
		pub fn get_schedules_by_task_id(
			key: ObjectId,
		) -> Result<Vec<TaskSchedule<T::AccountId>>, DispatchError> {
			let data = ScheduleStorage::<T>::iter_values()
				.filter(|item| item.task_id == key)
				.collect::<Vec<_>>();

			Ok(data)
		}
		pub fn get_schedules_keys() -> Result<Vec<u64>, DispatchError> {
			let data_list = ScheduleStorage::<T>::iter_keys().collect::<Vec<_>>();

			Ok(data_list)
		}
		pub fn get_schedule_by_key(
			key: u64,
		) -> Result<Option<TaskSchedule<T::AccountId>>, DispatchError> {
			let data = ScheduleStorage::<T>::get(key);

			Ok(data)
		}

		pub fn get_payable_task_schedules(
		) -> Result<PayableScheduleResults<T::AccountId>, DispatchError> {
			let data_list = PayableScheduleStorage::<T>::iter()
				.filter(|item| item.1.status == ScheduleStatus::Initiated)
				.collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_payable_schedules_by_task_id(
			key: ObjectId,
		) -> Result<Vec<PayableTaskSchedule<T::AccountId>>, DispatchError> {
			let data = PayableScheduleStorage::<T>::iter_values()
				.filter(|item| item.task_id == key)
				.collect::<Vec<_>>();

			Ok(data)
		}

		fn ocw_update_schedule_by_key(data: OCWSkdData) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();

			if let Some((acc, res)) =
				signer.send_signed_transaction(|_account| Call::update_schedule {
					status: data.status.clone(),
					key: data.key,
				}) {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
					return Err(Error::OffchainSignedTxFailed);
				} else {
					log::info!("success: offchain_signed_tx: tx sent: {:?}", acc.id);
					return Ok(());
				}
			}

			log::error!("No local account available");
			Err(Error::NoLocalAcctForSignedTx)
		}
	}

	pub trait ScheduleFetchInterface<AccountId> {
		fn get_schedule_via_task_id(
			key: ObjectId,
		) -> Result<Vec<TaskSchedule<AccountId>>, DispatchError>;
		fn get_payable_schedules_via_task_id(
			key: ObjectId,
		) -> Result<Vec<PayableTaskSchedule<AccountId>>, DispatchError>;
	}

	impl<T: Config> ScheduleFetchInterface<T::AccountId> for Pallet<T> {
		fn get_schedule_via_task_id(
			key: ObjectId,
		) -> Result<Vec<TaskSchedule<T::AccountId>>, DispatchError> {
			Self::get_schedules_by_task_id(key)
		}

		fn get_payable_schedules_via_task_id(
			key: ObjectId,
		) -> Result<Vec<PayableTaskSchedule<T::AccountId>>, DispatchError> {
			Self::get_payable_schedules_by_task_id(key)
		}
	}
}
