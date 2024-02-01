#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub use pallet::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod rewards;
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
	use schnorr_evm::VerifyingKey;
	use sp_runtime::{
		traits::{AccountIdConversion, IdentifyAccount, Zero},
		Saturating,
	};
	use sp_std::vec;
	use sp_std::vec::Vec;
	use time_primitives::{
		append_hash_with_task_data, AccountId, Balance, DepreciationRate, Function, Network,
		RewardConfig, ShardId, ShardsInterface, TaskCycle, TaskDescriptor, TaskDescriptorParams,
		TaskError, TaskExecution, TaskId, TaskPhase, TaskResult, TaskStatus, TasksInterface,
		TssSignature,
	};

	pub trait WeightInfo {
		fn create_task(input_length: u64) -> Weight;
		fn stop_task() -> Weight;
		fn resume_task() -> Weight;
		fn submit_result() -> Weight;
		fn submit_error() -> Weight;
		fn submit_hash(input_length: u64) -> Weight;
		fn submit_signature() -> Weight;
		fn register_gateway() -> Weight;
		fn fund_task() -> Weight;
		fn set_read_task_reward() -> Weight;
		fn set_write_task_reward() -> Weight;
		fn set_send_message_task_reward() -> Weight;
	}

	impl WeightInfo for () {
		fn create_task(_: u64) -> Weight {
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

		fn submit_hash(_: u64) -> Weight {
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

		fn set_write_task_reward() -> Weight {
			Weight::default()
		}

		fn set_send_message_task_reward() -> Weight {
			Weight::default()
		}
	}

	type BalanceOf<T> = <T as pallet_balances::Config>::Balance;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountId = AccountId> + pallet_balances::Config<Balance = Balance>
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Shards: ShardsInterface;
		#[pallet::constant]
		type MaxRetryCount: Get<u8>;
		/// Minimum task balance for tasks to payout expected rewards
		#[pallet::constant]
		type MinTaskBalance: Get<BalanceOf<Self>>;
		/// Base read task reward (for all networks)
		#[pallet::constant]
		type BaseReadReward: Get<BalanceOf<Self>>;
		/// Base write task reward (for all networks)
		#[pallet::constant]
		type BaseWriteReward: Get<BalanceOf<Self>>;
		/// Base send message task reward (for all networks)
		#[pallet::constant]
		type BaseSendMessageReward: Get<BalanceOf<Self>>;
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
	#[pallet::getter(fn network_read_reward)]
	pub type NetworkReadReward<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn network_write_reward)]
	pub type NetworkWriteReward<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn network_send_message_reward)]
	pub type NetworkSendMessageReward<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_reward_config)]
	pub type TaskRewardConfig<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		TaskId,
		RewardConfig<BalanceOf<T>, BlockNumberFor<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn member_payout)]
	pub type MemberPayout<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TaskId,
		Blake2_128Concat,
		ShardId,
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn signer_payout)]
	pub type SignerPayout<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TaskId,
		Blake2_128Concat,
		AccountId,
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn total_payout)]
	pub type TotalPayout<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, BalanceOf<T>, ValueQuery>;

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
		/// Write task reward set for network
		WriteTaskRewardSet(Network, BalanceOf<T>),
		/// Send message task reward set for network
		SendMessageTaskRewardSet(Network, BalanceOf<T>),
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
		#[pallet::weight(<T as Config>::WeightInfo::create_task(schedule.function.get_input_length()))]
		pub fn create_task(origin: OriginFor<T>, schedule: TaskDescriptorParams) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::start_task(schedule, Some(who))?;
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::stop_task())]
		pub fn stop_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
			let (owner, _) = Self::ensure_owner(origin, task_id)?;
			let task_account = Self::task_account(task_id);
			let task_balance = pallet_balances::Pallet::<T>::free_balance(&task_account);
			pallet_balances::Pallet::<T>::transfer(
				&task_account,
				&owner,
				task_balance,
				ExistenceRequirement::AllowDeath,
			)?;
			ensure!(
				TaskState::<T>::get(task_id) == Some(TaskStatus::Created),
				Error::<T>::InvalidTaskState
			);
			TaskState::<T>::insert(task_id, TaskStatus::Stopped);
			Self::deposit_event(Event::TaskStopped(task_id));
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::resume_task())]
		pub fn resume_task(
			origin: OriginFor<T>,
			task_id: TaskId,
			start: u64,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let (owner, task) = Self::ensure_owner(origin, task_id)?;
			ensure!(
				Self::is_resumable_post_transfer(task_id, amount),
				Error::<T>::InvalidTaskState
			);
			Self::transfer_to_task(&owner, task_id, amount)?;
			Self::do_resume_task(task_id, task, start);
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_result())]
		pub fn submit_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			result: TaskResult,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
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
			Self::validate_signature(
				task_id,
				cycle,
				result.shard_id,
				result.hash,
				result.signature,
			)?;
			ensure!(TaskCycleState::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			if Self::snapshot_rewards(task_id, result.shard_id, caller) {
				// persist task stoppage due to task balance below total payout
				return Ok(());
			}
			TaskCycleState::<T>::insert(task_id, cycle.saturating_plus_one());
			TaskResults::<T>::insert(task_id, cycle, result.clone());
			TaskRetryCounter::<T>::insert(task_id, 0);
			if Self::is_complete(task_id) {
				if let Some(shard_id) = TaskShard::<T>::take(task_id) {
					ShardTasks::<T>::remove(shard_id, task_id);
				}
				// payout all rewards due for the task at the very end upon completion
				Self::payout_task_rewards(task_id);
				TaskRewardConfig::<T>::remove(task_id);
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
		#[pallet::weight(<T as Config>::WeightInfo::submit_error())]
		pub fn submit_error(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			error: TaskError,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			Self::validate_signature(
				task_id,
				cycle,
				error.shard_id,
				VerifyingKey::message_hash(error.msg.as_bytes()),
				error.signature,
			)?;
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
		#[pallet::weight(<T as Config>::WeightInfo::submit_hash(hash.len() as u64))]
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
			if Self::task_balance(task_id) < T::MinTaskBalance::get() {
				// Stop task execution in read phase until funded sufficiently
				TaskState::<T>::insert(task_id, TaskStatus::Stopped);
				Self::deposit_event(Event::TaskStopped(task_id));
			}
			ReadPhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number());
			TaskPhaseState::<T>::insert(task_id, TaskPhase::Read(Some(hash)));
			Ok(())
		}

		/// Submit Signature
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_signature())] // TODO update bench, weights
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
		#[pallet::weight(<T as Config>::WeightInfo::register_gateway())] // TODO update bench, weights
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
		#[pallet::weight(<T as Config>::WeightInfo::fund_task())]
		pub fn fund_task(
			origin: OriginFor<T>,
			task_id: TaskId,
			amount: BalanceOf<T>,
			start: u64,
		) -> DispatchResult {
			let (owner, task) = Self::ensure_owner(origin, task_id)?;
			Self::transfer_to_task(&owner, task_id, amount)?;
			if Self::is_resumable(task_id) {
				Self::do_resume_task(task_id, task, start);
			}
			Ok(())
		}

		/// Set read task reward
		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::set_read_task_reward())]
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

		/// Set write task reward
		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::set_write_task_reward())]
		pub fn set_write_task_reward(
			origin: OriginFor<T>,
			network: Network,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			NetworkWriteReward::<T>::insert(network, amount);
			Self::deposit_event(Event::WriteTaskRewardSet(network, amount));
			Ok(())
		}

		/// Set send message task reward
		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::set_send_message_task_reward())]
		pub fn set_send_message_task_reward(
			origin: OriginFor<T>,
			network: Network,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			NetworkSendMessageReward::<T>::insert(network, amount);
			Self::deposit_event(Event::SendMessageTaskRewardSet(network, amount));
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
			pallet_balances::Pallet::<T>::free_balance(&Self::task_account(task_id))
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
			pallet_balances::Pallet::<T>::transfer(
				caller,
				&task_account_id,
				amount,
				ExistenceRequirement::KeepAlive,
			)?;
			Self::deposit_event(Event::TaskFunded(
				task_id,
				pallet_balances::Pallet::<T>::free_balance(&task_account_id),
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
		fn start_task(schedule: TaskDescriptorParams, who: Option<AccountId>) -> DispatchResult {
			ensure!(schedule.cycle > 0, Error::<T>::CycleMustBeGreaterThanZero);
			let task_id = TaskIdCounter::<T>::get();
			if let Some(funded_by) = &who {
				// transfer schedule.funds to task funded account
				pallet_balances::Pallet::<T>::transfer(
					funded_by,
					&Self::task_account(task_id),
					schedule.funds,
					ExistenceRequirement::KeepAlive,
				)?;
			} // else task is unfunded until `fund_task` is called?
			if schedule.function.is_gmp() {
				TaskPhaseState::<T>::insert(task_id, TaskPhase::Sign);
			}
			// Snapshot the reward config in storage
			TaskRewardConfig::<T>::insert(
				task_id,
				RewardConfig {
					read_task_reward: T::BaseReadReward::get()
						+ NetworkReadReward::<T>::get(schedule.network),
					write_task_reward: T::BaseWriteReward::get()
						+ NetworkWriteReward::<T>::get(schedule.network),
					send_message_reward: T::BaseSendMessageReward::get()
						+ NetworkSendMessageReward::<T>::get(schedule.network),
					depreciation_rate: T::RewardDeclineRate::get(),
				},
			);
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
					shard_size: schedule.shard_size,
				},
			);
			TaskState::<T>::insert(task_id, TaskStatus::Created);
			TaskIdCounter::<T>::put(task_id.saturating_plus_one());
			UnassignedTasks::<T>::insert(schedule.network, task_id, ());
			Self::deposit_event(Event::TaskCreated(task_id));
			Self::schedule_tasks(schedule.network);
			Ok(())
		}

		/// Ensure origin is signed and signature corresponds to TaskId owner
		/// Returns Ok(owner, task) iff origin is signed and task exists
		fn ensure_owner(
			origin: OriginFor<T>,
			task_id: TaskId,
		) -> Result<(T::AccountId, TaskDescriptor), DispatchError> {
			let caller = ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			let owner = task.owner.clone().ok_or(Error::<T>::InvalidOwner)?;
			ensure!(owner == caller, Error::<T>::InvalidOwner);
			Ok((owner, task))
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
					let task_balance_post_transfer =
						Self::task_balance(task_id).saturating_add(add);
					// to be resumed from stopped state:
					// 1. must be able to afford executing payout
					task_balance_post_transfer > TotalPayout::<T>::get(task_id)
					// 2. must be have >= min task balance
					&& task_balance_post_transfer >= T::MinTaskBalance::get()
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
			task_id: TaskId,
			task_cycle: TaskCycle,
			shard_id: ShardId,
			hash: [u8; 32],
			signature: TssSignature,
		) -> DispatchResult {
			let data = append_hash_with_task_data(hash, task_id, task_cycle);
			let hash = VerifyingKey::message_hash(&data);
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
				let Some(TaskDescriptor { shard_size, .. }) = Tasks::<T>::get(task_id) else {
					// this branch should never be hit, maybe turn this into expect
					continue;
				};
				let Some(shard_id) = Self::select_shard(network, task_id, None, shard_size) else {
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
			shard_size: u32,
		) -> Option<ShardId> {
			NetworkShards::<T>::iter_prefix(network)
				.filter(|(shard_id, _)| T::Shards::is_shard_online(*shard_id))
				.filter(|(shard_id, _)| {
					let shards_len = T::Shards::shard_members(*shard_id).len() as u32;
					shards_len == shard_size
				})
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
			let Some(TaskDescriptor { network, shard_size, .. }) = Tasks::<T>::get(task_id) else {
				// unexpected branch: network not found logic bug if this branch is hit
				return;
			};
			let Some(new_shard_id) =
				Self::select_shard(network, task_id, Some(old_shard_id), shard_size)
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

		/// Apply the depreciation rate
		fn apply_depreciation(
			start: BlockNumberFor<T>,
			amount: BalanceOf<T>,
			rate: DepreciationRate<BlockNumberFor<T>>,
		) -> BalanceOf<T> {
			let time_since_start = frame_system::Pallet::<T>::block_number().saturating_sub(start);
			if time_since_start.is_zero() {
				// no time elapsed since read phase started => full reward
				return amount;
			}
			let mut remaining = amount;
			let periods = time_since_start / rate.blocks;
			let mut i = BlockNumberFor::<T>::zero();
			while i < periods {
				remaining = remaining.saturating_sub(rate.percent * remaining);
				i = i.saturating_plus_one();
			}
			remaining
		}

		/// Compute task reward due to write task submitter
		fn compute_write_reward(task_id: TaskId) -> BalanceOf<T> {
			let Some(RewardConfig {
				write_task_reward,
				depreciation_rate,
				..
			}) = TaskRewardConfig::<T>::get(task_id)
			else {
				// reward config never stored => bug edge case
				return BalanceOf::<T>::zero();
			};
			Self::apply_depreciation(
				WritePhaseStart::<T>::get(task_id),
				write_task_reward,
				depreciation_rate,
			)
		}

		fn compute_read_reward(task_id: TaskId) -> BalanceOf<T> {
			let Some(RewardConfig {
				read_task_reward,
				depreciation_rate,
				..
			}) = TaskRewardConfig::<T>::get(task_id)
			else {
				// reward config never stored, bug edge case
				return BalanceOf::<T>::zero();
			};
			Self::apply_depreciation(
				ReadPhaseStart::<T>::get(task_id),
				read_task_reward,
				depreciation_rate,
			)
		}

		fn compute_send_message_reward(task_id: TaskId) -> BalanceOf<T> {
			let Some(RewardConfig {
				send_message_reward,
				depreciation_rate,
				..
			}) = TaskRewardConfig::<T>::get(task_id)
			else {
				// reward config never stored, bug edge case
				return BalanceOf::<T>::zero();
			};
			let start = match TaskPhaseState::<T>::get(task_id) {
				TaskPhase::Read(_) => ReadPhaseStart::<T>::get(task_id),
				TaskPhase::Write(_) => WritePhaseStart::<T>::get(task_id),
				_ => return BalanceOf::<T>::zero(),
			};
			Self::apply_depreciation(start, send_message_reward, depreciation_rate)
		}

		/// Snapshots rewards in storage to be paid out once task is complete
		/// Return true if task stoppage should be persisted due to low
		/// task balance
		fn snapshot_rewards(task_id: TaskId, shard_id: ShardId, caller: AccountId) -> bool {
			// payout to all shard members if possible
			let Some(TaskDescriptor { function, .. }) = Tasks::<T>::get(task_id) else {
				return false;
			};
			let mut task_reward_per_member = if function.is_gmp() {
				Self::compute_send_message_reward(task_id)
			} else {
				BalanceOf::<T>::zero()
			};
			let new_payout = match TaskPhaseState::<T>::get(task_id) {
				TaskPhase::Read(_) => {
					// add read task reward to shard member payout
					task_reward_per_member =
						task_reward_per_member.saturating_add(Self::compute_read_reward(task_id));
					task_reward_per_member
						.saturating_mul((T::Shards::shard_members(shard_id).len() as u32).into())
				},
				TaskPhase::Write(_) => {
					let old_signer_payout = SignerPayout::<T>::get(task_id, &caller);
					let new_signer_reward = Self::compute_write_reward(task_id);
					SignerPayout::<T>::insert(
						task_id,
						caller,
						old_signer_payout.saturating_add(new_signer_reward),
					);
					task_reward_per_member
						.saturating_mul((T::Shards::shard_members(shard_id).len() as u32).into())
						.saturating_add(new_signer_reward)
				},
				_ => return false, // skip reward payout altogether
			};
			let old_member_payout = MemberPayout::<T>::get(task_id, shard_id);
			MemberPayout::<T>::insert(
				task_id,
				shard_id,
				old_member_payout.saturating_add(task_reward_per_member),
			);
			let old_total_payout = TotalPayout::<T>::get(task_id);
			let new_total_payout = old_total_payout.saturating_add(new_payout);
			TotalPayout::<T>::insert(task_id, new_total_payout);
			if new_total_payout > Self::task_balance(task_id) {
				// stop task if new total payout exceeds task balance
				TaskState::<T>::insert(task_id, TaskStatus::Stopped);
				Self::deposit_event(Event::TaskStopped(task_id));
				return true;
			}
			// update phase start for next reward payout
			match TaskPhaseState::<T>::get(task_id) {
				TaskPhase::Read(_) => {
					ReadPhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number())
				},
				TaskPhase::Write(_) => {
					WritePhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number())
				},
				_ => (),
			}
			false
		}

		fn payout_task_rewards(task_id: TaskId) {
			let task_account_id = Self::task_account(task_id);
			for (shard_id, payout) in MemberPayout::<T>::drain_prefix(task_id) {
				// payout to shard members
				let members = T::Shards::shard_members(shard_id);
				for member in members {
					let _ = pallet_balances::Pallet::<T>::transfer(
						&task_account_id,
						&member,
						payout,
						ExistenceRequirement::AllowDeath,
					);
				}
			}
			for (account, payout) in SignerPayout::<T>::drain_prefix(task_id) {
				let _ = pallet_balances::Pallet::<T>::transfer(
					&task_account_id,
					&account,
					payout,
					ExistenceRequirement::AllowDeath,
				);
			}
			TotalPayout::<T>::remove(task_id);
		}
	}

	impl<T: Config> TasksInterface for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::insert(network, shard_id, ());
			Self::start_task(
				TaskDescriptorParams::new(
					network,
					Function::RegisterShard { shard_id },
					T::Shards::shard_members(shard_id).len() as u32,
				),
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
					TaskDescriptorParams::new(
						network,
						Function::UnregisterShard { shard_id },
						T::Shards::shard_members(shard_id).len() as u32,
					),
					None,
				)
				.unwrap();
			} else {
				Self::schedule_tasks(network);
			}
		}
	}
}
