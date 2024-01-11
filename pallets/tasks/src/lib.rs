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
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AccountIdConversion, IdentifyAccount, Zero},
		Percent, Saturating,
	};
	use sp_std::vec;
	use sp_std::vec::Vec;
	use time_primitives::{
		append_hash_with_task_data, AccountId, Function, Network, ShardId, ShardsInterface,
		TaskCycle, TaskDescriptor, TaskDescriptorParams, TaskError, TaskExecution, TaskId,
		TaskPhase, TaskResult, TaskStatus, TasksInterface, TssSignature,
	};

	pub trait WeightInfo {
		fn create_task() -> Weight;
		fn stop_task() -> Weight;
		fn resume_task() -> Weight;
		fn submit_result() -> Weight;
		fn submit_error() -> Weight;
		fn submit_hash() -> Weight;
		fn submit_signature() -> Weight;
		fn register_gateway() -> Weight;
		fn fund_task() -> Weight;
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

		fn submit_signature() -> Weight {
			Weight::default()
		}

		fn register_gateway() -> Weight {
			Weight::default()
		}

		fn fund_task() -> Weight {
			Weight::default()
		}
	}

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Shards: ShardsInterface;
		/// The currency type
		type Currency: Currency<Self::AccountId>;
		#[pallet::constant]
		type MaxRetryCount: Get<u8>;
		/// Minimum task balance for tasks in the READ phase (to pay out reward)
		#[pallet::constant]
		type MinReadTaskBalance: Get<BalanceOf<Self>>;
		/// Minimum and default read task reward for all networks
		#[pallet::constant]
		type MinReadTaskReward: Get<BalanceOf<Self>>;
		/// Reward declines every N blocks since read task start by Percent * Amount
		#[pallet::constant]
		type RewardDeclineRate: Get<(BlockNumberFor<Self>, Percent)>;
		#[pallet::constant]
		type WritePhaseTimeout: Get<BlockNumberFor<Self>>;
		/// `PalletId` for this pallet, used to derive an account for each task.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	#[pallet::storage]
	pub type UnassignedTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, Network, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	pub type ShardTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_shard)]
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
	#[pallet::getter(fn task_state)]
	pub type TaskState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskStatus, OptionQuery>;

	#[pallet::storage]
	pub type TaskPhaseState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskPhase, ValueQuery>;

	#[pallet::storage]
	pub type TaskSignature<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TssSignature, OptionQuery>;

	#[pallet::storage]
	pub type WritePhaseStart<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	pub type ReadPhaseStart<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, BlockNumberFor<T>, ValueQuery>;

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
		TaskResult,
		OptionQuery,
	>;

	#[pallet::storage]
	pub type ShardRegistered<T: Config> = StorageMap<_, Blake2_128Concat, ShardId, (), OptionQuery>;

	#[pallet::storage]
	pub type Gateway<T: Config> = StorageMap<_, Blake2_128Concat, Network, Vec<u8>, OptionQuery>;

	#[pallet::storage]
	pub type ReadTaskReward<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, BalanceOf<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Updated cycle status
		TaskResult(TaskId, TaskCycle, TaskResult),
		/// Task failed due to more errors than max retry count
		TaskFailed(TaskId, TaskCycle, TaskError),
		/// Task stopped by owner
		TaskStopped(TaskId),
		/// Task resumed by owner
		TaskResumed(TaskId),
		/// Gateway registered on network
		GatewayRegistered(Network, Vec<u8>),
		/// Task funded with new free balance amount
		TaskFunded(TaskId, BalanceOf<T>),
		/// Read task reward set for network
		ReadTaskRewardSet(Network, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Task
		UnknownTask,
		/// Unknown Shard
		UnknownShard,
		/// Invalid Signature
		InvalidSignature,
		/// Invalid Task State
		InvalidTaskState,
		/// Invalid Owner
		InvalidOwner,
		/// Invalid cycle
		InvalidCycle,
		/// Cycle must be greater than zero
		CycleMustBeGreaterThanZero,
		/// Not sign phase
		NotSignPhase,
		/// Not write phase
		NotWritePhase,
		/// Invalid signer
		InvalidSigner,
		/// Task not assigned
		UnassignedTask,
		/// Task already signed
		TaskSigned,
		/// Cannot submit result for task stopped
		TaskStopped,
		/// Cannot submit result for GMP functions unless gateway is registered
		GatewayNotRegistered,
		/// Bootstrap shard must be online to call register_gateway
		BootstrapShardMustBeOnline,
		/// Read task reward must be >= MinReadTaskReward
		ReadTaskRewardBelowPalletMin,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_task())]
		pub fn create_task(
			origin: OriginFor<T>,
			schedule: TaskDescriptorParams<BalanceOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::start_task(schedule, Some(who))?;
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::stop_task())]
		pub fn stop_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
			Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(Self::try_root_or_owner(origin, task_id), Error::<T>::InvalidOwner);
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
		pub fn resume_task(origin: OriginFor<T>, task_id: TaskId, start: u64) -> DispatchResult {
			let mut task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(Self::try_root_or_owner(origin, task_id), Error::<T>::InvalidOwner);
			ensure!(Self::is_resumable(task_id), Error::<T>::InvalidTaskState);
			task.start = start;
			Tasks::<T>::insert(task_id, task);
			TaskState::<T>::insert(task_id, TaskStatus::Created);
			TaskRetryCounter::<T>::insert(task_id, 0);
			Self::deposit_event(Event::TaskResumed(task_id));
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::submit_result())]
		pub fn submit_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			result: TaskResult,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			let status = TaskState::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			if TaskResults::<T>::get(task_id, cycle).is_some()
				|| matches!(status, TaskStatus::Completed)
			{
				return Ok(());
			}
			if task.function.is_gmp() {
				ensure!(
					Gateway::<T>::get(task.network).is_some(),
					Error::<T>::GatewayNotRegistered
				);
			}
			ensure!(!matches!(status, TaskStatus::Stopped), Error::<T>::TaskStopped);
			Self::validate_signature(result.shard_id, result.hash, result.signature)?;
			ensure!(TaskCycleState::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			if Self::reward_if_read_task_or_return_early(task_id, result.shard_id, task.network) {
				return Ok(());
			} // else reward payout succeeded or task is not in read phase
			TaskCycleState::<T>::insert(task_id, cycle.saturating_plus_one());
			TaskResults::<T>::insert(task_id, cycle, result.clone());
			TaskRetryCounter::<T>::insert(task_id, 0);
			if Self::is_complete(task_id) {
				if let Some(shard_id) = TaskShard::<T>::take(task_id) {
					ShardTasks::<T>::remove(shard_id, task_id);
				}
				TaskState::<T>::insert(task_id, TaskStatus::Completed);
				if let Function::RegisterShard { shard_id } = task.function {
					ShardRegistered::<T>::insert(shard_id, ());
				}
			}
			Self::deposit_event(Event::TaskResult(task_id, cycle, result));
			Ok(())
		}

		/// Submit Task Error
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::submit_error())]
		pub fn submit_error(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			error: TaskError,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			let hash = append_hash_with_task_data(error.msg.clone().into_bytes(), task_id, cycle);
			let final_hash = schnorr_evm::VerifyingKey::message_hash(&hash);
			Self::validate_signature(error.shard_id, final_hash, error.signature)?;
			ensure!(TaskCycleState::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			let retry_count = TaskRetryCounter::<T>::get(task_id);
			let new_retry_count = retry_count.saturating_plus_one();
			TaskRetryCounter::<T>::insert(task_id, new_retry_count);
			// task fails when new retry count == max - 1 => old retry count == max
			if new_retry_count == T::MaxRetryCount::get() {
				TaskState::<T>::insert(task_id, TaskStatus::Failed { error: error.clone() });
				Self::deposit_event(Event::TaskFailed(task_id, cycle, error));
			}
			Ok(())
		}

		/// Submit Task Hash
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::submit_hash())]
		pub fn submit_hash(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			hash: Vec<u8>,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			ensure!(TaskCycleState::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			let TaskPhase::Write(public_key) = TaskPhaseState::<T>::get(task_id) else {
				return Err(Error::<T>::NotWritePhase.into());
			};
			ensure!(signer == public_key.into_account(), Error::<T>::InvalidSigner);
			WritePhaseStart::<T>::remove(task_id);
			if Self::task_balance(task_id) < T::MinReadTaskBalance::get() {
				// Stop task execution in read phase until funded sufficiently
				TaskState::<T>::insert(task_id, TaskStatus::Stopped);
			}
			ReadPhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number());
			TaskPhaseState::<T>::insert(task_id, TaskPhase::Read(Some(hash)));
			Ok(())
		}

		/// Submit Signature
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::submit_signature())] // TODO update bench, weights
		pub fn submit_signature(
			origin: OriginFor<T>,
			task_id: TaskId,
			signature: TssSignature,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(TaskSignature::<T>::get(task_id).is_none(), Error::<T>::TaskSigned);
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			let TaskPhase::Sign = TaskPhaseState::<T>::get(task_id) else {
				return Err(Error::<T>::NotSignPhase.into());
			};
			let Some(shard_id) = TaskShard::<T>::get(task_id) else {
				return Err(Error::<T>::UnassignedTask.into());
			};
			Self::start_write_phase(task_id, shard_id);
			TaskSignature::<T>::insert(task_id, signature);
			Ok(())
		}

		/// Register gateway
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::register_gateway())] // TODO update bench, weights
		pub fn register_gateway(
			origin: OriginFor<T>,
			bootstrap: ShardId,
			address: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(T::Shards::is_shard_online(bootstrap), Error::<T>::BootstrapShardMustBeOnline);
			let network = T::Shards::shard_network(bootstrap).ok_or(Error::<T>::UnknownShard)?;
			for (id, state) in Tasks::<T>::iter() {
				if let Function::RegisterShard { shard_id } = state.function {
					if bootstrap == shard_id
						&& matches!(TaskState::<T>::get(id), Some(TaskStatus::Created))
					{
						TaskState::<T>::insert(id, TaskStatus::Completed);
					}
				}
			}
			ShardRegistered::<T>::insert(bootstrap, ());
			Gateway::<T>::insert(network, address.clone());
			Self::schedule_tasks(network);
			Self::deposit_event(Event::GatewayRegistered(network, address));
			Ok(())
		}

		/// Fund task balance
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::fund_task())]
		pub fn fund_task(
			origin: OriginFor<T>,
			task_id: TaskId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let task_account_id = Self::task_account(task_id);
			let task_state = TaskState::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			T::Currency::transfer(
				&caller,
				&task_account_id,
				amount,
				ExistenceRequirement::KeepAlive,
			)?;
			let new_task_balance = T::Currency::free_balance(&task_account_id);
			if matches!(task_state, TaskStatus::Stopped) {
				if new_task_balance >= T::MinReadTaskBalance::get() {
					// RESUME the task
					TaskState::<T>::insert(task_id, TaskStatus::Created);
				}
			}
			Self::deposit_event(Event::TaskFunded(task_id, new_task_balance));
			Ok(())
		}

		/// Set read task reward
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::fund_task())] //Update
		pub fn set_read_task_reward(
			origin: OriginFor<T>,
			network: Network,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				amount >= T::MinReadTaskReward::get(),
				Error::<T>::ReadTaskRewardBelowPalletMin
			);
			ReadTaskReward::<T>::insert(network, amount);
			Self::deposit_event(Event::ReadTaskRewardSet(network, amount));
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut writes = 0;
			WritePhaseStart::<T>::iter().for_each(|(task_id, created_block)| {
				if n.saturating_sub(created_block) >= T::WritePhaseTimeout::get() {
					if let Some(shard_id) = TaskShard::<T>::get(task_id) {
						Self::start_write_phase(task_id, shard_id);
						writes += 2;
					}
				}
			});
			T::DbWeight::get().writes(writes)
		}
	}

	impl<T: Config> Pallet<T> {
		/// The account ID containing the funds for a task.
		pub fn task_account(task_id: TaskId) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(task_id)
		}
		/// Balance for a task
		pub fn task_balance(task_id: TaskId) -> BalanceOf<T> {
			T::Currency::free_balance(&Self::task_account(task_id))
		}
		fn start_task(
			schedule: TaskDescriptorParams<BalanceOf<T>>,
			who: Option<AccountId>,
		) -> DispatchResult {
			ensure!(schedule.cycle > 0, Error::<T>::CycleMustBeGreaterThanZero);
			let task_id = TaskIdCounter::<T>::get();
			if let Some(funded_by) = &who {
				// transfer schedule.funds to task funded account
				T::Currency::transfer(
					funded_by,
					&Self::task_account(task_id),
					schedule.funds,
					ExistenceRequirement::KeepAlive,
				)?;
			} // else task is unfunded until `fund_task` is called?
			if schedule.function.is_gmp() {
				TaskPhaseState::<T>::insert(task_id, TaskPhase::Sign);
			}
			Tasks::<T>::insert(
				task_id,
				TaskDescriptor {
					owner: who,
					network: schedule.network,
					function: schedule.function,
					cycle: schedule.cycle,
					start: schedule.start,
					period: schedule.period,
					timegraph: schedule.timegraph,
				},
			);
			TaskState::<T>::insert(task_id, TaskStatus::Created);
			TaskIdCounter::<T>::put(task_id.saturating_plus_one());
			UnassignedTasks::<T>::insert(schedule.network, task_id, ());
			Self::deposit_event(Event::TaskCreated(task_id));
			Self::schedule_tasks(schedule.network);
			Ok(())
		}

		fn try_root_or_owner(origin: OriginFor<T>, task_id: TaskId) -> bool {
			ensure_root(origin.clone()).is_ok()
				|| Tasks::<T>::get(task_id).map_or(false, |task| {
					if let Ok(origin) = ensure_signed(origin) {
						task.owner == Some(origin)
					} else {
						false
					}
				})
		}

		fn start_write_phase(task_id: TaskId, shard_id: ShardId) {
			TaskPhaseState::<T>::insert(
				task_id,
				TaskPhase::Write(T::Shards::random_signer(shard_id)),
			);
			WritePhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number());
		}

		pub fn get_task_signature(task: TaskId) -> Option<TssSignature> {
			TaskSignature::<T>::get(task)
		}

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

		pub fn get_gateway(network: Network) -> Option<Vec<u8>> {
			Gateway::<T>::get(network)
		}

		fn is_complete(task_id: TaskId) -> bool {
			if let Some(task) = Tasks::<T>::get(task_id) {
				TaskResults::<T>::contains_key(task_id, task.cycle.saturating_less_one())
			} else {
				true
			}
		}

		fn is_resumable(task_id: TaskId) -> bool {
			match TaskState::<T>::get(task_id) {
				Some(TaskStatus::Failed { .. }) => true,
				Some(TaskStatus::Stopped) => {
					if matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Read(_)) {
						// only resumable if task balance > min
						Self::task_balance(task_id) >= T::MinReadTaskBalance::get()
					} else {
						true
					}
				},
				_ => false,
			}
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

		fn validate_signature(
			shard_id: ShardId,
			hash: [u8; 32],
			signature: TssSignature,
		) -> DispatchResult {
			let public_key = T::Shards::tss_public_key(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let signature = schnorr_evm::Signature::from_bytes(signature)
				.map_err(|_| Error::<T>::InvalidSignature)?;
			let schnorr_public_key = schnorr_evm::VerifyingKey::from_bytes(public_key)
				.map_err(|_| Error::<T>::UnknownShard)?;
			schnorr_public_key
				.verify_prehashed(hash, &signature)
				.map_err(|_| Error::<T>::InvalidSignature)?;
			Ok(())
		}

		fn shard_task_count(shard_id: ShardId) -> usize {
			ShardTasks::<T>::iter_prefix(shard_id).count()
		}

		fn schedule_tasks(network: Network) {
			for (task_id, _) in UnassignedTasks::<T>::iter_prefix(network) {
				let shard = NetworkShards::<T>::iter_prefix(network)
					.filter(|(shard_id, _)| T::Shards::is_shard_online(*shard_id))
					.filter(|(shard_id, _)| {
						if TaskPhaseState::<T>::get(task_id) == TaskPhase::Sign {
							if Gateway::<T>::get(network).is_some() {
								ShardRegistered::<T>::get(*shard_id).is_some()
							} else {
								false
							}
						} else {
							true
						}
					})
					.map(|(shard_id, _)| (shard_id, Self::shard_task_count(shard_id)))
					.reduce(|(shard_id, task_count), (shard_id2, task_count2)| {
						if task_count < task_count2 {
							(shard_id, task_count)
						} else {
							(shard_id2, task_count2)
						}
					});
				let Some((shard_id, _)) = shard else {
					// on gmp task sometimes returns none and it stops every other schedule
					continue;
				};

				if Self::is_payable(task_id)
					&& !matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Read(Some(_)))
					&& !matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Sign)
				{
					Self::start_write_phase(task_id, shard_id);
				}
				ShardTasks::<T>::insert(shard_id, task_id, ());
				TaskShard::<T>::insert(task_id, shard_id);
				UnassignedTasks::<T>::remove(network, task_id);
			}
		}

		pub fn get_task_cycle(task_id: TaskId) -> TaskCycle {
			TaskCycleState::<T>::get(task_id)
		}

		pub fn get_task_phase(task_id: TaskId) -> TaskPhase {
			TaskPhaseState::<T>::get(task_id)
		}

		pub fn get_task_shard(task_id: TaskId) -> Option<ShardId> {
			TaskShard::<T>::get(task_id)
		}

		pub fn get_task_results(
			task_id: TaskId,
			cycle: Option<TaskCycle>,
		) -> Vec<(TaskCycle, TaskResult)> {
			if let Some(cycle) = cycle {
				let mut results = vec![];
				if let Some(result) = TaskResults::<T>::get(task_id, cycle) {
					results.push((cycle, result))
				}
				results
			} else {
				TaskResults::<T>::iter_prefix(task_id).collect::<Vec<_>>()
			}
		}

		fn read_task_reward(network: Network) -> BalanceOf<T> {
			let read_task_reward = ReadTaskReward::<T>::get(network);
			if read_task_reward.is_zero() {
				let min_read_task_reward = T::MinReadTaskReward::get();
				// set to minimum read task reward is not set
				ReadTaskReward::<T>::insert(network, min_read_task_reward);
				min_read_task_reward
			} else {
				read_task_reward
			}
		}

		fn compute_shard_member_task_reward(network: Network, task_id: TaskId) -> BalanceOf<T> {
			let full_reward = Self::read_task_reward(network);
			let read_phase_start = ReadPhaseStart::<T>::get(task_id);
			if read_phase_start.is_zero() {
				// read phase never started => no reward
				return BalanceOf::<T>::zero();
			}
			let (discount_period_len, discount_pct) = T::RewardDeclineRate::get();
			let time_since_read_phase_start =
				frame_system::Pallet::<T>::block_number().saturating_sub(read_phase_start);
			if time_since_read_phase_start.is_zero() {
				// no time elapsed since read phase started => full reward
				return full_reward;
			}
			let mut remaining_reward = full_reward;
			let discount_periods = time_since_read_phase_start / discount_period_len;
			let mut discount_period_counter = BlockNumberFor::<T>::zero();
			while discount_period_counter < discount_periods {
				let amt_to_discount = discount_pct * remaining_reward;
				remaining_reward = remaining_reward.saturating_sub(amt_to_discount);
				discount_period_counter = discount_period_counter.saturating_plus_one();
			}
			remaining_reward
		}

		/// Returns true iff task is read and payout fails which requires
		/// returning early.
		/// Returns false if task is not in read phase OR if payout succeeded.
		// TODO: clean/refactor
		fn reward_if_read_task_or_return_early(
			task_id: TaskId,
			shard_id: ShardId,
			network: Network,
		) -> bool {
			if !matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Read(_)) {
				// do NOT return early if task is not in read phase
				return false;
			} // else task is in read phase => reward the shard members
			let task_reward_per_member = Self::compute_shard_member_task_reward(network, task_id);
			let task_account_id = Self::task_account(task_id);
			let members = T::Shards::shard_members(shard_id);
			let stop_task = |t| {
				// insufficient balance to payout rewards
				TaskState::<T>::insert(t, TaskStatus::Stopped);
				Self::deposit_event(Event::TaskStopped(t));
			};
			if T::Currency::free_balance(&task_account_id)
				< task_reward_per_member.saturating_mul((members.len() as u32).into())
			{
				// insufficient balance to payout rewards
				stop_task(task_id);
				return true;
			}
			for member in T::Shards::shard_members(shard_id) {
				if T::Currency::transfer(
					&task_account_id,
					&member,
					task_reward_per_member,
					ExistenceRequirement::KeepAlive,
				)
				.is_err()
				{
					// insufficient balance to payout rewards
					// unlikely due to check above the loop
					stop_task(task_id);
					return true;
				} // else continue payout
			}
			// payout succeeded => do NOT return early in caller
			// update read phase start because read task completed and rewarded
			ReadPhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number());
			false
		}
	}

	impl<T: Config> TasksInterface for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::insert(network, shard_id, ());
			Self::start_task(
				TaskDescriptorParams::new(network, Function::RegisterShard { shard_id }),
				None,
			)
			.unwrap();
		}

		fn shard_offline(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::remove(network, shard_id);
			ShardTasks::<T>::drain_prefix(shard_id).for_each(|(task_id, _)| {
				TaskShard::<T>::remove(task_id);
				if Self::is_runnable(task_id) || Self::is_resumable(task_id) {
					UnassignedTasks::<T>::insert(network, task_id, ());
				}
			});
			for (id, state) in Tasks::<T>::iter() {
				if let Function::RegisterShard { shard_id: x } = state.function {
					if x == shard_id && matches!(TaskState::<T>::get(id), Some(TaskStatus::Created))
					{
						TaskState::<T>::insert(id, TaskStatus::Stopped);
						break;
					}
				}
			}
			if ShardRegistered::<T>::take(shard_id).is_some() {
				Self::start_task(
					TaskDescriptorParams::new(network, Function::UnregisterShard { shard_id }),
					None,
				)
				.unwrap();
			} else {
				Self::schedule_tasks(network);
			}
		}
	}
}
