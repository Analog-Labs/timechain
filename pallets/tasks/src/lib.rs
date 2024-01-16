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
		Saturating,
	};
	use sp_std::vec;
	use sp_std::vec::Vec;
	use time_primitives::{
		append_hash_with_task_data, AccountId, DepreciationRate, Function, Network, ShardId,
		ShardsInterface, TaskCycle, TaskDescriptor, TaskDescriptorParams, TaskError, TaskExecution,
		TaskId, TaskPhase, TaskResult, TaskStatus, TasksInterface, TssSignature,
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
		fn set_read_task_reward() -> Weight;
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

		fn set_read_task_reward() -> Weight {
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
		/// Base read task reward (for all networks)
		#[pallet::constant]
		type BaseReadReward: Get<BalanceOf<Self>>;
		/// Reward declines every N blocks since read task start by Percent * Amount
		#[pallet::constant]
		type RewardDeclineRate: Get<DepreciationRate<BlockNumberFor<Self>>>;
		#[pallet::constant]
		type WritePhaseTimeout: Get<BlockNumberFor<Self>>;
		#[pallet::constant]
		type ReadPhaseTimeout: Get<BlockNumberFor<Self>>;
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
	pub type NetworkReadReward<T: Config> =
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
			let tasks = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			let maybe_owner = if let Some(caller) = Self::try_root_or_owner(origin, task_id)? {
				Some(caller)
			} else {
				tasks.owner
			};
			if let Some(owner) = maybe_owner {
				// return task balance to owner
				let task_account = Self::task_account(task_id);
				let task_balance = T::Currency::free_balance(&task_account);
				T::Currency::transfer(
					&task_account,
					&owner,
					task_balance,
					ExistenceRequirement::AllowDeath,
				)?;
			} // else owner was NOT set => task balance NOT drained anywhere
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
		pub fn resume_task(
			origin: OriginFor<T>,
			task_id: TaskId,
			start: u64,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(
				Self::is_resumable_post_transfer(task_id, amount),
				Error::<T>::InvalidTaskState
			);
			if let Some(owner) = Self::try_root_or_owner(origin, task_id)? {
				Self::transfer_to_task(&owner, task_id, amount)?;
			}
			Self::do_resume_task(task_id, task, start);
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
			start: u64,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			Self::transfer_to_task(&caller, task_id, amount)?;
			if Self::is_resumable(task_id) {
				Self::do_resume_task(task_id, task, start);
			}
			Ok(())
		}

		/// Set read task reward
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::set_read_task_reward())]
		pub fn set_read_task_reward(
			origin: OriginFor<T>,
			network: Network,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			NetworkReadReward::<T>::insert(network, amount);
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
			ReadPhaseStart::<T>::iter().for_each(|(task_id, created_block)| {
				if n.saturating_sub(created_block) >= T::ReadPhaseTimeout::get() {
					Self::schedule_task_to_new_shard(task_id);
					writes += 3
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

		/// Returns Ok(task_should_be_resumed: bool) so
		/// Ok(true) if task needs to be resumed
		/// Ok(false) if not
		fn transfer_to_task(
			caller: &T::AccountId,
			task_id: TaskId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let task_account_id = Self::task_account(task_id);
			T::Currency::transfer(
				caller,
				&task_account_id,
				amount,
				ExistenceRequirement::KeepAlive,
			)?;
			Self::deposit_event(Event::TaskFunded(
				task_id,
				T::Currency::free_balance(&task_account_id),
			));
			Ok(())
		}

		fn do_resume_task(task_id: TaskId, task: TaskDescriptor, start: u64) {
			Tasks::<T>::insert(task_id, TaskDescriptor { start, ..task });
			TaskState::<T>::insert(task_id, TaskStatus::Created);
			TaskRetryCounter::<T>::insert(task_id, 0);
			Self::deposit_event(Event::TaskResumed(task_id));
		}

		/// Start task
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

		/// Returns Ok(None) if root, Ok(Some(owner)) if owner, and Err otherwise
		fn try_root_or_owner(
			origin: OriginFor<T>,
			task_id: TaskId,
		) -> Result<Option<T::AccountId>, DispatchError> {
			if ensure_root(origin.clone()).is_ok() {
				// origin is root
				return Ok(None);
			} // else try ensure_signed to check if caller is task owner
			let caller = ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			ensure!(task.owner == Some(caller.clone()), Error::<T>::InvalidOwner);
			// origin is owner
			Ok(Some(caller))
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

		/// Check if task is resumable after `add` is added to its balance
		fn is_resumable_post_transfer(task_id: TaskId, add: BalanceOf<T>) -> bool {
			match TaskState::<T>::get(task_id) {
				Some(TaskStatus::Failed { .. }) => true,
				Some(TaskStatus::Stopped) => {
					if matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Read(_)) {
						println!("x");
						Self::task_balance(task_id).saturating_add(add)
							>= T::MinReadTaskBalance::get()
					} else {
						println!("x");
						true
					}
				},
				_ => false,
			}
		}

		fn is_resumable(task_id: TaskId) -> bool {
			Self::is_resumable_post_transfer(task_id, BalanceOf::<T>::zero())
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
				let Some(shard_id) = Self::select_shard(network, task_id, None) else {
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

		/// Select shard for task assignment
		/// Selects the shard for the input Network with the least number of tasks
		/// assigned to it.
		/// Excludes selecting the `old` shard_id optional input if it is passed
		/// for task reassignment purposes.
		fn select_shard(
			network: Network,
			task_id: TaskId,
			old: Option<ShardId>,
		) -> Option<ShardId> {
			NetworkShards::<T>::iter_prefix(network)
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
				.filter(|(shard_id, _)| {
					if let Some(previous_shard) = old {
						shard_id != &previous_shard
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
				})
				.map(|(s, _)| s)
		}

		fn schedule_task_to_new_shard(task_id: TaskId) {
			let Some(old_shard_id) = TaskShard::<T>::get(task_id) else {
				// task not assigned to any existing shard so nothing changes
				return;
			};
			let Some(TaskDescriptor { network, .. }) = Tasks::<T>::get(task_id) else {
				// unexpected branch: network not found logic bug if this branch is hit
				return;
			};
			let Some(new_shard_id) = Self::select_shard(network, task_id, Some(old_shard_id))
			else {
				// no new shard available for network for task reassignment
				return;
			};
			if matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Read(_)) {
				// reset read phase start for task (and consequently the reward)
				ReadPhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number());
			}
			ShardTasks::<T>::remove(old_shard_id, task_id);
			ShardTasks::<T>::insert(new_shard_id, task_id, ());
			TaskShard::<T>::insert(task_id, new_shard_id);
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

		fn compute_shard_member_task_reward(network: Network, task_id: TaskId) -> BalanceOf<T> {
			let full_reward =
				NetworkReadReward::<T>::get(network).saturating_add(T::BaseReadReward::get());
			let read_phase_start = ReadPhaseStart::<T>::get(task_id);
			if read_phase_start.is_zero() {
				// read phase never started => no reward
				return BalanceOf::<T>::zero();
			}
			let rate = T::RewardDeclineRate::get();
			let time_since_read_phase_start =
				frame_system::Pallet::<T>::block_number().saturating_sub(read_phase_start);
			if time_since_read_phase_start.is_zero() {
				// no time elapsed since read phase started => full reward
				return full_reward;
			}
			let mut remaining_reward = full_reward;
			let periods = time_since_read_phase_start / rate.blocks;
			let mut i = BlockNumberFor::<T>::zero();
			while i < periods {
				remaining_reward = remaining_reward.saturating_sub(rate.percent * remaining_reward);
				i = i.saturating_plus_one();
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
