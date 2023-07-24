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
	use time_primitives::abstraction::TaskMetadataInterface;
	use time_primitives::{
		abstraction::{
			OCWSkdData, ObjectId, PayableScheduleInput, PayableTaskSchedule, ScheduleInput,
			ScheduleStatus, TaskSchedule,
		},
		scheduling::GetNetworkTimeout,
		sharding::{EligibleShard, HandleShardTasks, IncrementTaskTimeoutCount, Network},
		PalletAccounts, ProxyExtend, OCW_SKD_KEY,
	};

	pub(crate) type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type KeyId = u64;
	pub type ScheduleResults<AccountId, BlockNumber> =
		Vec<(KeyId, TaskSchedule<AccountId, BlockNumber>)>;
	pub type PayableScheduleResults<AccountId, BlockNumber> =
		Vec<(KeyId, PayableTaskSchedule<AccountId, BlockNumber>)>;
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
			let current_block = frame_system::Pallet::<T>::block_number();
			let mut timed_out_tasks = TimedOutTasks::<T>::get();
			for (schedule_id, schedule) in <ScheduleStorage<T>>::iter() {
				if schedule.status.can_timeout()
					&& !timed_out_tasks.contains(&schedule_id)
					&& current_block.saturating_sub(schedule.executable_since)
						>= T::RecurringTimeoutLength::get_network_timeout(schedule.network)
				{
					timed_out_tasks.push(schedule_id);
					if let Some(assigned_shard) = TaskAssignedShard::<T>::get(schedule_id) {
						T::ShardTimeouts::increment_task_timeout_count(assigned_shard);
					}
				}
			}
			for (schedule_id, schedule) in <PayableScheduleStorage<T>>::iter() {
				if schedule.status.can_timeout()
					&& !timed_out_tasks.contains(&schedule_id)
					&& current_block.saturating_sub(schedule.executable_since)
						>= T::PayableTimeoutLength::get_network_timeout(schedule.network)
				{
					timed_out_tasks.push(schedule_id);
					if let Some(assigned_shard) = TaskAssignedShard::<T>::get(schedule_id) {
						T::ShardTimeouts::increment_task_timeout_count(assigned_shard);
					}
				}
			}
			TimedOutTasks::<T>::put(timed_out_tasks);
			if T::ShouldEndSession::should_end_session(now) {
				// TODO check if we should reward the indexer once or continue reward history data
				// otherwise we can drain all data at the end of epoch
				for (indexer, times) in IndexerScore::<T>::drain() {
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
				T::DbWeight::get().reads(1)
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
		type ShardEligibility: EligibleShard<u64, Network>;
		type ShardTimeouts: IncrementTaskTimeoutCount<u64>;
		type RecurringTimeoutLength: GetNetworkTimeout<Network, Self::BlockNumber>;
		type PayableTimeoutLength: GetNetworkTimeout<Network, Self::BlockNumber>;
		type TaskMetadataHelper: TaskMetadataInterface;
	}

	#[pallet::storage]
	#[pallet::getter(fn timed_out_tasks)]
	pub type TimedOutTasks<T: Config> = StorageValue<_, Vec<KeyId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn unassigned_tasks)]
	pub type UnassignedTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, KeyId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_tasks)]
	pub type ShardTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, u64, Blake2_128Concat, KeyId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_assigned_shard)]
	pub type TaskAssignedShard<T: Config> =
		StorageMap<_, Blake2_128Concat, KeyId, u64, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_schedule)]
	pub type ScheduleStorage<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		KeyId,
		TaskSchedule<T::AccountId, T::BlockNumber>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_payable_task_schedule)]
	pub type PayableScheduleStorage<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		KeyId,
		PayableTaskSchedule<T::AccountId, T::BlockNumber>,
		OptionQuery,
	>;

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
		///Offchain signed tx failed
		OffchainSignedTxFailed,
		///no local account for signed tx
		NoLocalAcctForSignedTx,
		/// Shard cannot be assigned tasks due to ineligibility
		ShardNotEligibleForTasks,
		/// Task not assigned to any shards
		TaskNotAssigned,
		/// Task is assigned so cannot be reassigned
		TaskAssigned,
		/// Task Metadata is not registered
		TaskMetadataNotRegistered,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::insert_schedule())]
		pub fn insert_schedule(origin: OriginFor<T>, schedule: ScheduleInput) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				T::TaskMetadataHelper::task_metadata_exists(schedule.task_id.get_id()),
				Error::<T>::TaskMetadataNotRegistered
			);
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
			// assign task to next eligible shard for this network
			if let Some(next_shard_for_network) =
				T::ShardEligibility::next_eligible_shard(schedule.network)
			{
				Self::assign_task_to_shard(schedule_id, next_shard_for_network);
			} else {
				// place in unassigned tasks if no shards available for this network
				UnassignedTasks::<T>::insert(schedule.network, schedule_id, ());
			}
			ScheduleStorage::<T>::insert(
				schedule_id,
				TaskSchedule {
					task_id: schedule.task_id,
					owner: who,
					network: schedule.network,
					cycle: schedule.cycle,
					frequency: schedule.frequency,
					start_execution_block: 0,
					executable_since: frame_system::Pallet::<T>::block_number(),
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
			ScheduleStorage::<T>::try_mutate(key, |schedule| -> DispatchResult {
				let details = schedule.as_mut().ok_or(Error::<T>::ErrorRef)?;
				ensure!(details.owner == who, Error::<T>::NoPermission);

				details.status = status;
				Ok(())
			})?;
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
			ensure!(
				T::TaskMetadataHelper::task_metadata_exists(schedule.task_id.get_id()),
				Error::<T>::TaskMetadataNotRegistered
			);
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
			// assign task to next eligible shard for this network
			if let Some(next_shard_for_network) =
				T::ShardEligibility::next_eligible_shard(schedule.network)
			{
				Self::assign_task_to_shard(schedule_id, next_shard_for_network);
			} else {
				// place in unassigned tasks if no shards available for this network
				UnassignedTasks::<T>::insert(schedule.network, schedule_id, ());
			}
			PayableScheduleStorage::<T>::insert(
				schedule_id,
				PayableTaskSchedule {
					task_id: schedule.task_id,
					owner: who,
					network: schedule.network,
					executable_since: frame_system::Pallet::<T>::block_number(),
					status: ScheduleStatus::Initiated,
				},
			);
			Self::deposit_event(Event::PayableScheduleStored(schedule_id));

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		fn reset_task_execution_start_time(task: KeyId) {
			if let Some(mut schedule) = ScheduleStorage::<T>::get(task) {
				schedule.executable_since = frame_system::Pallet::<T>::block_number();
				ScheduleStorage::<T>::insert(task, schedule);
			} else if let Some(mut schedule) = PayableScheduleStorage::<T>::get(task) {
				schedule.executable_since = frame_system::Pallet::<T>::block_number();
				PayableScheduleStorage::<T>::insert(task, schedule);
			}
		}
		fn assign_task_to_shard(task: KeyId, shard: u64) {
			ShardTasks::<T>::insert(shard, task, ());
			TaskAssignedShard::<T>::insert(task, shard);
		}
		pub fn increment_indexer_reward_count(indexer: T::AccountId) -> Result<(), DispatchError> {
			IndexerScore::<T>::mutate(indexer, |reward| *reward += 1);
			Ok(())
		}

		pub fn get_task_shard(task: KeyId) -> Result<u64, DispatchError> {
			TaskAssignedShard::<T>::get(task).ok_or(Error::<T>::TaskNotAssigned.into())
		}

		pub fn get_one_time_schedules(
		) -> Result<ScheduleResults<T::AccountId, T::BlockNumber>, DispatchError> {
			let data_list = ScheduleStorage::<T>::iter()
				.filter(|item| item.1.status == ScheduleStatus::Initiated)
				.filter(|item| !item.1.is_repetitive_task())
				.collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_repetitive_schedules(
		) -> Result<ScheduleResults<T::AccountId, T::BlockNumber>, DispatchError> {
			let data_list = ScheduleStorage::<T>::iter()
				.filter(|item| item.1.status == ScheduleStatus::Initiated)
				.filter(|item| item.1.is_repetitive_task())
				.collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_schedules_by_schedule_id(
			key: ObjectId,
		) -> Result<Vec<TaskSchedule<T::AccountId, T::BlockNumber>>, DispatchError> {
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
		) -> Result<Option<TaskSchedule<T::AccountId, T::BlockNumber>>, DispatchError> {
			let data = ScheduleStorage::<T>::get(key);

			Ok(data)
		}

		pub fn get_payable_task_schedules(
		) -> Result<PayableScheduleResults<T::AccountId, T::BlockNumber>, DispatchError> {
			let data_list = PayableScheduleStorage::<T>::iter()
				.filter(|item| item.1.status == ScheduleStatus::Initiated)
				.collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_payable_schedules_by_schedule_id(
			key: ObjectId,
		) -> Result<Vec<PayableTaskSchedule<T::AccountId, T::BlockNumber>>, DispatchError> {
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

		pub fn get_payable_schedule_by_key(
			key: u64,
		) -> Result<Option<PayableTaskSchedule<T::AccountId, T::BlockNumber>>, DispatchError> {
			let data = PayableScheduleStorage::<T>::get(key);

			Ok(data)
		}
	}

	impl<T: Config> HandleShardTasks<u64, Network, KeyId> for Pallet<T> {
		fn handle_shard_tasks(shard_id: u64, network: Network) {
			// move incomplete shard tasks to unassigned task queue
			let move_incomplete_tasks = |status, schedule_id| {
				if status != ScheduleStatus::Completed {
					ShardTasks::<T>::remove(shard_id, schedule_id);
					TaskAssignedShard::<T>::remove(schedule_id);
					UnassignedTasks::<T>::insert(network, schedule_id, ());
				}
			};
			ShardTasks::<T>::iter_prefix(shard_id).for_each(|(schedule_id, _)| {
				if let Some(schedule) = PayableScheduleStorage::<T>::get(schedule_id) {
					move_incomplete_tasks(schedule.status, schedule_id);
				} else if let Some(schedule) = ScheduleStorage::<T>::get(schedule_id) {
					move_incomplete_tasks(schedule.status, schedule_id);
				}
			});
		}

		fn claim_task_for_shard(
			shard_id: u64,
			network: Network,
			schedule_id: KeyId,
		) -> DispatchResult {
			ensure!(
				UnassignedTasks::<T>::take(network, schedule_id).is_some(),
				Error::<T>::TaskAssigned
			);
			Self::reset_task_execution_start_time(schedule_id);
			Self::assign_task_to_shard(schedule_id, shard_id);
			Ok(())
		}
	}

	pub trait ScheduleInterface<AccountId, BlockNumber> {
		fn get_assigned_shard_for_key(key: u64) -> Result<u64, DispatchError>;
		fn get_schedule_via_key(
			key: u64,
		) -> Result<Option<TaskSchedule<AccountId, BlockNumber>>, DispatchError>;
		fn get_payable_schedule_via_key(
			key: u64,
		) -> Result<Option<PayableTaskSchedule<AccountId, BlockNumber>>, DispatchError>;
		fn decrement_schedule_cycle(key: u64) -> Result<(), DispatchError>;
		fn update_completed_task(key: u64);
	}

	impl<T: Config> ScheduleInterface<T::AccountId, T::BlockNumber> for Pallet<T> {
		fn get_assigned_shard_for_key(key: u64) -> Result<u64, DispatchError> {
			TaskAssignedShard::<T>::get(key).ok_or(Error::<T>::TaskNotAssigned.into())
		}
		fn get_schedule_via_key(
			key: u64,
		) -> Result<Option<TaskSchedule<T::AccountId, T::BlockNumber>>, DispatchError> {
			Self::get_schedule_by_key(key)
		}

		fn get_payable_schedule_via_key(
			key: u64,
		) -> Result<Option<PayableTaskSchedule<T::AccountId, T::BlockNumber>>, DispatchError> {
			Self::get_payable_schedule_by_key(key)
		}

		fn decrement_schedule_cycle(key: u64) -> Result<(), DispatchError> {
			ScheduleStorage::<T>::try_mutate(key, |schedule| -> DispatchResult {
				let details = schedule.as_mut().ok_or(Error::<T>::ErrorRef)?;
				details.cycle = details.cycle.saturating_sub(1);
				Ok(())
			})?;
			Ok(())
		}

		fn update_completed_task(key: u64) {
			if let Some(mut schedule) = ScheduleStorage::<T>::get(key) {
				if schedule.is_repetitive_task() {
					schedule.executable_since = frame_system::Pallet::<T>::block_number();
					schedule.status = ScheduleStatus::Updated;
				} else {
					schedule.status = ScheduleStatus::Completed;
				}
				ScheduleStorage::<T>::insert(key, schedule);
			} else if let Some(mut schedule) = PayableScheduleStorage::<T>::get(key) {
				schedule.status = ScheduleStatus::Completed;
				PayableScheduleStorage::<T>::insert(key, schedule);
			}
			// no longer timed out upon completion
			TimedOutTasks::<T>::mutate(|tasks| tasks.retain(|t| *t != key));
		}
	}
}
