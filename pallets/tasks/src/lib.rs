#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub use pallet::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::string::String;
	use sp_runtime::{traits::IdentifyAccount, Saturating};
	use sp_std::vec;
	use sp_std::vec::Vec;
	use time_primitives::{
		CycleStatus, Network, PublicKey, ShardId, ShardsInterface, TaskCycle, TaskDescriptor,
		TaskDescriptorParams, TaskError, TaskExecution, TaskId, TaskPhase, TaskStatus,
		TasksInterface,
	};

	pub trait WeightInfo {
		fn create_task() -> Weight;
		fn stop_task() -> Weight;
		fn resume_task() -> Weight;
		fn submit_result() -> Weight;
		fn submit_error() -> Weight;
		fn submit_hash() -> Weight;
	}

	impl WeightInfo for () {
		fn create_task() -> Weight {
			Weight::default()
		}

		fn stop_task() -> Weight {
			Weight::default()
		}

		fn resume_task() -> Weight {
			Weight::default()
		}

		fn submit_result() -> Weight {
			Weight::default()
		}

		fn submit_error() -> Weight {
			Weight::default()
		}

		fn submit_hash() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<Call<Self>, Public = PublicKey>
		+ frame_system::Config<AccountId = sp_runtime::AccountId32>
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type WeightInfo: WeightInfo;
		type Shards: ShardsInterface;
		#[pallet::constant]
		type MaxRetryCount: Get<u8>;
	}

	#[pallet::storage]
	pub type UnassignedTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	pub type ShardTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	pub type TaskShard<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, ShardId, OptionQuery>;

	#[pallet::storage]
	pub type NetworkShards<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, ShardId, (), OptionQuery>;

	#[pallet::storage]
	pub type TaskIdCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tasks)]
	pub type Tasks<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskDescriptor, OptionQuery>;

	#[pallet::storage]
	pub type TaskState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskStatus, OptionQuery>;

	#[pallet::storage]
	pub type TaskPhaseState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskPhase, ValueQuery>;

	#[pallet::storage]
	pub type TaskRetryCounter<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, u8, ValueQuery>;

	#[pallet::storage]
	pub type TaskCycleState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskCycle, ValueQuery>;

	#[pallet::storage]
	pub type TaskResults<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TaskId,
		Blake2_128Concat,
		TaskCycle,
		CycleStatus,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Updated cycle status
		TaskResult(TaskId, TaskCycle, CycleStatus),
		/// Task failed due to more errors than max retry count
		TaskFailed(TaskId, TaskError),
		/// Task stopped by owner
		TaskStopped(TaskId),
		/// Task resumed by owner
		TaskResumed(TaskId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Task
		UnknownTask,
		/// Invalid Task State
		InvalidTaskState,
		/// Invalid Owner
		InvalidOwner,
		/// Invalid cycle
		InvalidCycle,
		/// Cycle must be greater than zero
		CycleMustBeGreaterThanZero,
		/// Collector peer id not found
		CollectorPeerIdNotFound,
		// Tx is not signed by collector of shard
		NotSignedByCollector,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_task())]
		pub fn create_task(origin: OriginFor<T>, schedule: TaskDescriptorParams) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(schedule.cycle > 0, Error::<T>::CycleMustBeGreaterThanZero);
			let task_id = TaskIdCounter::<T>::get();
			Tasks::<T>::insert(
				task_id,
				TaskDescriptor {
					owner: who,
					network: schedule.network,
					function: schedule.function,
					cycle: schedule.cycle,
					start: schedule.start,
					period: schedule.period,
					hash: schedule.hash,
				},
			);
			TaskState::<T>::insert(task_id, TaskStatus::Created);
			TaskIdCounter::<T>::put(task_id.saturating_plus_one());
			UnassignedTasks::<T>::insert(schedule.network, task_id, ());
			Self::deposit_event(Event::TaskCreated(task_id));
			Self::schedule_tasks(schedule.network);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::stop_task())]
		pub fn stop_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(task.owner == owner, Error::<T>::InvalidOwner);
			ensure!(
				TaskState::<T>::get(task_id) == Some(TaskStatus::Created),
				Error::<T>::InvalidTaskState
			);
			TaskState::<T>::insert(task_id, TaskStatus::Stopped);
			Self::deposit_event(Event::TaskStopped(task_id));
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::resume_task())]
		pub fn resume_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(task.owner == owner, Error::<T>::InvalidOwner);
			ensure!(
				TaskState::<T>::get(task_id) == Some(TaskStatus::Stopped),
				Error::<T>::InvalidTaskState
			);
			TaskState::<T>::insert(task_id, TaskStatus::Created);
			Self::deposit_event(Event::TaskResumed(task_id));
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::submit_result())]
		pub fn submit_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			status: CycleStatus,
		) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(TaskCycleState::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			TaskCycleState::<T>::insert(task_id, cycle.saturating_plus_one());
			TaskResults::<T>::insert(task_id, cycle, status.clone());
			TaskRetryCounter::<T>::insert(task_id, 0);
			if Self::is_complete(task_id) {
				if let Some(shard_id) = TaskShard::<T>::take(task_id) {
					ShardTasks::<T>::remove(shard_id, task_id);
				}
				TaskState::<T>::insert(task_id, TaskStatus::Completed);
			}
			Self::deposit_event(Event::TaskResult(task_id, cycle, status));
			Ok(())
		}

		/// Submit Task Error
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::submit_error())]
		pub fn submit_error(
			origin: OriginFor<T>,
			task_id: TaskId,
			error: TaskError,
		) -> DispatchResult {
			ensure_none(origin)?;
			let retry_count = TaskRetryCounter::<T>::get(task_id);
			TaskRetryCounter::<T>::insert(task_id, retry_count.saturating_plus_one());
			// task fails when new retry count == max - 1 => old retry count == max
			if retry_count == T::MaxRetryCount::get() {
				TaskState::<T>::insert(task_id, TaskStatus::Failed { error: error.clone() });
				Self::deposit_event(Event::TaskFailed(task_id, error));
			}
			Ok(())
		}

		/// Submit Task Hash
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::submit_hash())]
		pub fn submit_hash(
			origin: OriginFor<T>,
			shard_id: ShardId,
			task_id: TaskId,
			hash: String,
		) -> DispatchResult {
			Self::ensure_signed_by_collector(origin, shard_id)?;
			ensure!(Tasks::<T>::get(task_id).is_some(), (Error::<T>::UnknownTask));
			ensure!(TaskShard::<T>::get(task_id) == Some(shard_id), Error::<T>::InvalidOwner);
			TaskPhaseState::<T>::insert(task_id, TaskPhase::Read(Some(hash)));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution> {
			ShardTasks::<T>::iter_prefix(shard_id)
				.filter(|(task_id, _)| Self::is_runnable(*task_id))
				.map(|(task_id, _)| {
					TaskExecution::new(
						task_id,
						TaskCycleState::<T>::get(task_id),
						TaskRetryCounter::<T>::get(task_id),
						TaskPhaseState::<T>::get(task_id),
					)
				})
				.collect()
		}

		pub fn get_task(task_id: TaskId) -> Option<TaskDescriptor> {
			Tasks::<T>::get(task_id)
		}

		fn ensure_signed_by_collector(origin: OriginFor<T>, shard_id: ShardId) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let Some(collector) = T::Shards::collector_pubkey(shard_id) else {
				return Err(Error::<T>::NotSignedByCollector.into());
			};
			ensure!(account_id == collector.into_account(), Error::<T>::NotSignedByCollector);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn is_complete(task_id: TaskId) -> bool {
			if let Some(task) = Tasks::<T>::get(task_id) {
				TaskResults::<T>::contains_key(task_id, task.cycle.saturating_less_one())
			} else {
				true
			}
		}

		fn is_resumable(task_id: TaskId) -> bool {
			matches!(TaskState::<T>::get(task_id), Some(TaskStatus::Stopped))
		}

		fn is_runnable(task_id: TaskId) -> bool {
			matches!(TaskState::<T>::get(task_id), Some(TaskStatus::Created))
		}

		fn is_payable(task_id: TaskId) -> bool {
			let Some(task) = Self::get_task(task_id) else {
				return false;
			};
			task.function.is_payable()
		}

		fn shard_task_count(shard_id: ShardId) -> usize {
			ShardTasks::<T>::iter_prefix(shard_id).count()
		}

		fn schedule_tasks(network: Network) {
			for (task_id, _) in UnassignedTasks::<T>::iter_prefix(network) {
				let shard = NetworkShards::<T>::iter_prefix(network)
					.filter(|(shard_id, _)| T::Shards::is_shard_online(*shard_id))
					.filter_map(|(shard_id, _)| {
						T::Shards::collector_peer_id(shard_id)
							.map(|collector| (shard_id, collector))
					})
					.map(|(shard_id, collector)| {
						(shard_id, Self::shard_task_count(shard_id), collector)
					})
					.reduce(
						|(shard_id, task_count, collector),
						 (shard_id2, task_count2, collector2)| {
							if task_count < task_count2 {
								(shard_id, task_count, collector)
							} else {
								(shard_id2, task_count2, collector2)
							}
						},
					);
				let Some((shard_id, _, collector)) = shard else {
					break;
				};

				if Self::is_payable(task_id)
					&& !matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Read(Some(_)))
				{
					TaskPhaseState::<T>::insert(task_id, TaskPhase::Write(collector))
				}
				ShardTasks::<T>::insert(shard_id, task_id, ());
				TaskShard::<T>::insert(task_id, shard_id);
				UnassignedTasks::<T>::remove(network, task_id);
			}
		}

		pub fn submit_task_hash(shard_id: ShardId, task_id: TaskId, hash: String) {
			let Some(collector) = T::Shards::collector_pubkey(shard_id) else {
				log::error!("No collector found while sumbitting task hash {:?}", task_id);
				return;
			};
			let signer = Signer::<T, T::AuthorityId>::any_account().with_filter(vec![collector]);

			let call_res = signer.send_signed_transaction(|_| Call::submit_hash {
				shard_id,
				task_id,
				hash: hash.clone(),
			});

			let Some((_, res)) = call_res else {
				log::info!("send signed transaction returned none");
				return;
			};
			if let Err(e) = res {
				log::error!("send signed transaction returned an error: {:?}", e)
			}
		}

		pub fn submit_task_result(task_id: TaskId, cycle: TaskCycle, status: CycleStatus) {
			use frame_system::offchain::SubmitTransaction;

			let call = Call::submit_result { task_id, cycle, status };
			let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
				.map_err(|_| {
					log::error!("Failed to submit task result {:?}/{:?}", task_id, cycle);
				});
		}

		pub fn submit_task_error(task_id: TaskId, error: TaskError) {
			use frame_system::offchain::SubmitTransaction;

			let call = Call::submit_error { task_id, error };
			let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
				.map_err(|_| {
					log::error!("Failed to submit task error {:?}", task_id);
				});
		}
	}

	impl<T: Config> TasksInterface for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::insert(network, shard_id, ());
			Self::schedule_tasks(network);
		}

		fn shard_offline(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::remove(network, shard_id);
			ShardTasks::<T>::drain_prefix(shard_id).for_each(|(task_id, _)| {
				TaskShard::<T>::remove(task_id);
				if Self::is_runnable(task_id) || Self::is_resumable(task_id) {
					UnassignedTasks::<T>::insert(network, task_id, ());
				}
			});
			Self::schedule_tasks(network);
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		fn validate_unsigned(source: TransactionSource, _call: &Self::Call) -> TransactionValidity {
			if !matches!(source, TransactionSource::Local | TransactionSource::InBlock) {
				return InvalidTransaction::Call.into();
			}
			ValidTransaction::with_tag_prefix("tasks-pallet")
				.priority(TransactionPriority::max_value())
				.longevity(3)
				.propagate(true)
				.build()
		}

		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}
	}
}
