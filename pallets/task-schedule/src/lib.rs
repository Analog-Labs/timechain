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
		abstraction::{OCWSkdData, ScheduleInput, ScheduleStatus, TaskSchedule},
		sharding::{EligibleShard, HandleShardTasks, Network},
		PalletAccounts, ProxyExtend, ScheduleCycle, ShardId, TaskId, OCW_SKD_KEY,
	};

	pub(crate) type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub trait WeightInfo {
		fn insert_schedule() -> Weight;
		fn update_schedule() -> Weight;
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
				|res: Result<Option<VecDeque<Vec<u8>>>, StorageRetrievalError>| {
					match res {
						Ok(Some(mut data)) => {
							// iteration batch of 5
							for _ in 0..5 {
								let Some(data_vec) = data.pop_front() else {
									break;
								};

								let Ok(skd_req) = OCWSkdData::decode(&mut data_vec.as_slice()) else {
									continue;
								};

								if let Err(err) = Self::ocw_update_schedule_by_key(skd_req.clone())
								{
									log::error!(
										"Error occured while submitting extrinsic {:?}",
										err
									);
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
	pub trait Config:
		CreateSignedTransaction<Call<Self>>
		+ frame_system::Config<AccountId = sp_runtime::AccountId32>
	{
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
	}

	#[pallet::storage]
	#[pallet::getter(fn unassigned_tasks)]
	pub type UnassignedTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_tasks)]
	pub type ShardTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_assigned_shard)]
	pub type TaskAssignedShard<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, ShardId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_schedule)]
	pub type ScheduleStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskSchedule, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_cycle)]
	pub type TaskCycle<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, ScheduleCycle, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_result)]
	pub type TaskResults<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TaskId,
		Blake2_128Concat,
		ScheduleCycle,
		ScheduleStatus,
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
		ScheduleStored(TaskId),

		/// Updated Schedule
		ScheduleUpdated(TaskId, ScheduleCycle),

		///Already exist case
		AlreadyExist(TaskId),

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
		/// Invalid cycle
		InvalidCycle,
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
					owner: who,
					network: schedule.network,
					function: schedule.function,
					cycle: schedule.cycle,
					start: schedule.start,
					period: schedule.period,
					hash: schedule.hash,
				},
			);
			Self::deposit_event(Event::ScheduleStored(schedule_id));

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_schedule())]
		pub fn update_schedule(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: ScheduleCycle,
			status: ScheduleStatus,
			// proof: Signature, TODO: add proof to authenticate
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(TaskCycle::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			TaskCycle::<T>::insert(task_id, cycle + 1);
			TaskResults::<T>::insert(task_id, cycle, status);
			Self::deposit_event(Event::ScheduleUpdated(task_id, cycle));
			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		fn assign_task_to_shard(task: TaskId, shard: u64) {
			ShardTasks::<T>::insert(shard, task, ());
			TaskAssignedShard::<T>::insert(task, shard);
		}
		pub fn increment_indexer_reward_count(indexer: T::AccountId) -> Result<(), DispatchError> {
			IndexerScore::<T>::mutate(indexer, |reward| *reward += 1);
			Ok(())
		}

		pub fn get_schedules_keys() -> Result<Vec<u64>, DispatchError> {
			let data_list = ScheduleStorage::<T>::iter_keys().collect::<Vec<_>>();

			Ok(data_list)
		}

		pub fn get_task_via_id(task_id: TaskId) -> Option<TaskSchedule> {
			ScheduleStorage::<T>::get(task_id)
		}

		pub fn api_get_shard_tasks(shard_id: ShardId) -> Vec<(TaskId, ScheduleCycle)> {
			ShardTasks::<T>::iter_prefix(shard_id)
				.map(|(id, _)| (id, TaskCycle::<T>::get(id)))
				.collect::<Vec<_>>()
		}

		fn ocw_update_schedule_by_key(data: OCWSkdData) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();

			if let Some((acc, res)) =
				signer.send_signed_transaction(|_account| Call::update_schedule {
					task_id: data.task_id,
					cycle: data.cycle,
					status: data.status.clone(),
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

		fn is_task_complete(task_id: TaskId) -> bool {
			if let Some(task) = ScheduleStorage::<T>::get(task_id) {
				TaskResults::<T>::contains_key(task_id, task.cycle)
			} else {
				true
			}
		}
	}

	impl<T: Config> HandleShardTasks<ShardId, Network, TaskId> for Pallet<T> {
		fn handle_shard_tasks(shard_id: ShardId, network: Network) {
			ShardTasks::<T>::iter_prefix(shard_id).for_each(|(task_id, _)| {
				ShardTasks::<T>::remove(shard_id, task_id);
				TaskAssignedShard::<T>::remove(task_id);
				if !Self::is_task_complete(task_id) {
					UnassignedTasks::<T>::insert(network, task_id, ());
				}
			});
		}

		fn claim_task_for_shard(
			shard_id: ShardId,
			network: Network,
			task_id: TaskId,
		) -> DispatchResult {
			ensure!(
				UnassignedTasks::<T>::take(network, task_id).is_some(),
				Error::<T>::TaskAssigned
			);
			Self::assign_task_to_shard(task_id, shard_id);
			Ok(())
		}
	}

	pub trait ScheduleInterface<AccountId> {
		fn get_assigned_shard_for_key(task_id: TaskId) -> Result<ShardId, DispatchError>;
		fn get_assigned_schedule_count(shard: ShardId) -> usize;
	}

	impl<T: Config> ScheduleInterface<T::AccountId> for Pallet<T> {
		fn get_assigned_shard_for_key(task_id: TaskId) -> Result<ShardId, DispatchError> {
			TaskAssignedShard::<T>::get(task_id).ok_or(Error::<T>::TaskNotAssigned.into())
		}
		fn get_assigned_schedule_count(shard_id: ShardId) -> usize {
			ShardTasks::<T>::iter_prefix(shard_id).count()
		}
	}
}
