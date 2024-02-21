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
	use schnorr_evm::VerifyingKey;
	use sp_runtime::{
		traits::{AccountIdConversion, IdentifyAccount, Zero},
		Saturating,
	};
	use sp_std::vec;
	use sp_std::vec::Vec;
	use time_primitives::{
		append_hash_with_task_data, AccountId, Balance, DepreciationRate, Function, GmpParams,
		Message, NetworkId, RewardConfig, ShardId, ShardsInterface, TaskDescriptor,
		TaskDescriptorParams, TaskError, TaskExecution, TaskFunder, TaskId, TaskPhase, TaskResult,
		TaskStatus, TasksInterface, TransferStake, TssSignature,
	};

	pub trait WeightInfo {
		fn create_task(input_length: u64) -> Weight;
		fn submit_result() -> Weight;
		fn submit_error() -> Weight;
		fn submit_hash(input_length: u64) -> Weight;
		fn submit_signature() -> Weight;
		fn register_gateway() -> Weight;
		fn set_read_task_reward() -> Weight;
		fn set_write_task_reward() -> Weight;
		fn set_send_message_task_reward() -> Weight;
	}

	impl WeightInfo for () {
		fn create_task(_: u64) -> Weight {
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
		type Members: TransferStake;
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
		StorageDoubleMap<_, Blake2_128Concat, NetworkId, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	pub type ShardTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TaskId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_shard)]
	pub type TaskShard<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, ShardId, OptionQuery>;

	#[pallet::storage]
	pub type NetworkShards<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		NetworkId,
		Blake2_128Concat,
		ShardId,
		(),
		OptionQuery,
	>;

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
	pub type TaskOutput<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskResult, OptionQuery>;

	#[pallet::storage]
	pub type ShardRegistered<T: Config> = StorageMap<_, Blake2_128Concat, ShardId, (), OptionQuery>;

	#[pallet::storage]
	pub type Gateway<T: Config> = StorageMap<_, Blake2_128Concat, NetworkId, [u8; 20], OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn network_read_reward)]
	pub type NetworkReadReward<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn network_write_reward)]
	pub type NetworkWriteReward<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn network_send_message_reward)]
	pub type NetworkSendMessageReward<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, BalanceOf<T>, ValueQuery>;

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

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Task succeeded with result
		TaskResult(TaskId, TaskResult),
		/// Task failed with error
		TaskFailed(TaskId, TaskError),
		/// Gateway registered on network
		GatewayRegistered(NetworkId, [u8; 20]),
		/// Read task reward set for network
		ReadTaskRewardSet(NetworkId, BalanceOf<T>),
		/// Write task reward set for network
		WriteTaskRewardSet(NetworkId, BalanceOf<T>),
		/// Send message task reward set for network
		SendMessageTaskRewardSet(NetworkId, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Task
		UnknownTask,
		/// Unknown Shard
		UnknownShard,
		/// Invalid Signature
		InvalidSignature,
		/// Invalid Task Phase
		InvalidTaskPhase,
		/// Invalid Owner
		InvalidOwner,
		/// Invalid task function
		InvalidTaskFunction,
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
			Self::start_task(schedule, TaskFunder::Account(who))?;
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_result())]
		pub fn submit_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			result: TaskResult,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			let status = TaskState::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			if TaskOutput::<T>::get(task_id).is_some() || matches!(status, TaskStatus::Completed) {
				return Ok(());
			}
			ensure!(
				matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Read(_)),
				Error::<T>::InvalidTaskPhase
			);
			let is_gmp = if task.function.is_gmp() {
				ensure!(
					Gateway::<T>::get(task.network).is_some(),
					Error::<T>::GatewayNotRegistered
				);
				true
			} else {
				false
			};
			let modified_data = append_hash_with_task_data(result.hash, task_id);
			let sig_hash = VerifyingKey::message_hash(&modified_data);
			Self::validate_signature(result.shard_id, sig_hash, result.signature)?;
			TaskOutput::<T>::insert(task_id, result.clone());
			TaskState::<T>::insert(task_id, TaskStatus::Completed);
			if let Some(shard_id) = TaskShard::<T>::take(task_id) {
				ShardTasks::<T>::remove(shard_id, task_id);
			}
			Self::payout_task_rewards(task_id, result.shard_id, is_gmp);
			if let Function::RegisterShard { shard_id } = task.function {
				ShardRegistered::<T>::insert(shard_id, ());
			}
			Self::deposit_event(Event::TaskResult(task_id, result));
			Ok(())
		}

		/// Submit Task Error
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_error())]
		pub fn submit_error(
			origin: OriginFor<T>,
			task_id: TaskId,
			error: TaskError,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(
				matches!(TaskPhaseState::<T>::get(task_id), TaskPhase::Read(_)),
				Error::<T>::InvalidTaskPhase
			);
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			let error_hash = VerifyingKey::message_hash(error.msg.as_bytes());
			let modified_data = append_hash_with_task_data(error_hash, task_id);
			let sig_hash = VerifyingKey::message_hash(&modified_data);
			Self::validate_signature(error.shard_id, sig_hash, error.signature)?;
			// Task fails on first error submission (no retry counter post no recurring tasks)
			TaskState::<T>::insert(task_id, TaskStatus::Failed { error: error.clone() });
			Self::deposit_event(Event::TaskFailed(task_id, error));
			Ok(())
		}

		/// Submit Task Hash
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_hash(hash.len() as u64))]
		pub fn submit_hash(origin: OriginFor<T>, task_id: TaskId, hash: Vec<u8>) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			let TaskPhase::Write(public_key) = TaskPhaseState::<T>::get(task_id) else {
				return Err(Error::<T>::NotWritePhase.into());
			};
			ensure!(signer == public_key.into_account(), Error::<T>::InvalidSigner);
			Self::snapshot_write_reward(task_id, signer);
			ReadPhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number());
			TaskPhaseState::<T>::insert(task_id, TaskPhase::Read(Some(hash)));
			Ok(())
		}

		/// Submit Signature
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_signature())] // TODO update bench, weights
		pub fn submit_signature(
			origin: OriginFor<T>,
			task_id: TaskId,
			signature: TssSignature,
			chain_id: u64,
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
			let hash = Self::get_gmp_hash(task_id, shard_id, chain_id)?;
			let sig_hash = VerifyingKey::message_hash(&hash);
			Self::validate_signature(shard_id, sig_hash, signature)?;
			Self::start_write_phase(task_id, shard_id);
			TaskSignature::<T>::insert(task_id, signature);
			Ok(())
		}

		/// Register gateway
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::register_gateway())] // TODO update bench, weights
		pub fn register_gateway(
			origin: OriginFor<T>,
			bootstrap: ShardId,
			address: [u8; 20],
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
			Gateway::<T>::insert(network, address);
			Self::schedule_tasks(network);
			Self::deposit_event(Event::GatewayRegistered(network, address));
			Ok(())
		}

		/// Set read task reward
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::set_read_task_reward())]
		pub fn set_read_task_reward(
			origin: OriginFor<T>,
			network: NetworkId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			NetworkReadReward::<T>::insert(network, amount);
			Self::deposit_event(Event::ReadTaskRewardSet(network, amount));
			Ok(())
		}

		/// Set write task reward
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::set_write_task_reward())]
		pub fn set_write_task_reward(
			origin: OriginFor<T>,
			network: NetworkId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			NetworkWriteReward::<T>::insert(network, amount);
			Self::deposit_event(Event::WriteTaskRewardSet(network, amount));
			Ok(())
		}

		/// Set send message task reward
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::set_send_message_task_reward())]
		pub fn set_send_message_task_reward(
			origin: OriginFor<T>,
			network: NetworkId,
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

		/// Start task
		fn start_task(schedule: TaskDescriptorParams, who: TaskFunder) -> DispatchResult {
			let task_id = TaskIdCounter::<T>::get();
			let (read_task_reward, write_task_reward, send_message_reward) = (
				T::BaseReadReward::get() + NetworkReadReward::<T>::get(schedule.network),
				T::BaseWriteReward::get() + NetworkWriteReward::<T>::get(schedule.network),
				T::BaseSendMessageReward::get()
					+ NetworkSendMessageReward::<T>::get(schedule.network),
			);
			let mut required_funds = read_task_reward
				.saturating_mul(schedule.shard_size.into())
				.saturating_add(write_task_reward);
			let is_gmp = if schedule.function.is_gmp() {
				required_funds = required_funds
					.saturating_add(send_message_reward.saturating_mul(schedule.shard_size.into()));
				true
			} else {
				false
			};
			let owner = match who {
				TaskFunder::Account(user) => {
					let funds = if schedule.funds >= required_funds {
						schedule.funds
					} else {
						required_funds
					};
					pallet_balances::Pallet::<T>::transfer(
						&user,
						&Self::task_account(task_id),
						funds,
						ExistenceRequirement::KeepAlive,
					)?;
					Some(user)
				},
				TaskFunder::Shard(shard_id) => {
					let task_account = Self::task_account(task_id);
					let amount = required_funds.saturating_div(schedule.shard_size.into());
					for member in T::Shards::shard_members(shard_id) {
						T::Members::transfer_stake(&member, &task_account, amount)?;
					}
					None
				},
			};
			if is_gmp {
				TaskPhaseState::<T>::insert(task_id, TaskPhase::Sign);
			} else if !schedule.function.is_payable() {
				// Task phase state is TaskPhase::Read(None) == TaskPhase::default()
				// so TaskPhaseState stays default.
				// Still need to start the read phase timeout:
				ReadPhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number());
			} // else write phase is started in schedule_tasks if task.function.is_payable() which means is Evm::Deploy || Evm::Call
  // Snapshot the reward config in storage
			TaskRewardConfig::<T>::insert(
				task_id,
				RewardConfig {
					read_task_reward,
					write_task_reward,
					send_message_reward,
					depreciation_rate: T::RewardDeclineRate::get(),
				},
			);
			Tasks::<T>::insert(
				task_id,
				TaskDescriptor {
					owner,
					network: schedule.network,
					function: schedule.function,
					start: schedule.start,
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

		fn start_write_phase(task_id: TaskId, shard_id: ShardId) {
			TaskPhaseState::<T>::insert(
				task_id,
				TaskPhase::Write(T::Shards::next_signer(shard_id)),
			);
			WritePhaseStart::<T>::insert(task_id, frame_system::Pallet::<T>::block_number());
		}

		pub fn get_task_signature(task: TaskId) -> Option<TssSignature> {
			TaskSignature::<T>::get(task)
		}

		pub fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution> {
			ShardTasks::<T>::iter_prefix(shard_id)
				.filter(|(task_id, _)| Self::is_runnable(*task_id))
				.map(|(task_id, _)| TaskExecution::new(task_id, TaskPhaseState::<T>::get(task_id)))
				.collect()
		}

		pub fn get_task(task_id: TaskId) -> Option<TaskDescriptor> {
			Tasks::<T>::get(task_id)
		}

		pub fn get_gateway(network: NetworkId) -> Option<[u8; 20]> {
			Gateway::<T>::get(network)
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

		fn schedule_tasks(network: NetworkId) {
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
			network: NetworkId,
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

		pub fn get_task_phase(task_id: TaskId) -> TaskPhase {
			TaskPhaseState::<T>::get(task_id)
		}

		pub fn get_task_shard(task_id: TaskId) -> Option<ShardId> {
			TaskShard::<T>::get(task_id)
		}

		pub fn get_task_result(task_id: TaskId) -> Option<TaskResult> {
			TaskOutput::<T>::get(task_id)
		}

		/// Apply the depreciation rate
		fn apply_depreciation(
			start: BlockNumberFor<T>,
			amount: BalanceOf<T>,
			rate: DepreciationRate<BlockNumberFor<T>>,
		) -> BalanceOf<T> {
			let now = frame_system::Pallet::<T>::block_number();
			let time_since_start = now.saturating_sub(start);
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

		fn snapshot_write_reward(task_id: TaskId, signer: AccountId) {
			let Some(RewardConfig {
				write_task_reward,
				depreciation_rate,
				..
			}) = TaskRewardConfig::<T>::get(task_id)
			else {
				// reward config never stored => bug edge case
				return;
			};
			SignerPayout::<T>::insert(
				task_id,
				signer,
				Self::apply_depreciation(
					WritePhaseStart::<T>::take(task_id),
					write_task_reward,
					depreciation_rate,
				),
			);
		}

		fn payout_task_rewards(task_id: TaskId, shard_id: ShardId, is_gmp: bool) {
			let task_account_id = Self::task_account(task_id);
			let start = ReadPhaseStart::<T>::take(task_id);
			let shard_member_reward = if let Some(RewardConfig {
				read_task_reward,
				send_message_reward,
				depreciation_rate,
				..
			}) = TaskRewardConfig::<T>::take(task_id)
			{
				let read_reward =
					Self::apply_depreciation(start, read_task_reward, depreciation_rate.clone());
				let send_msg_reward =
					Self::apply_depreciation(start, send_message_reward, depreciation_rate);
				if is_gmp {
					read_reward.saturating_add(send_msg_reward)
				} else {
					read_reward
				}
			} else {
				// reward config never stored, bug edge case
				BalanceOf::<T>::zero()
			};
			// payout each member of the shard
			T::Shards::shard_members(shard_id).into_iter().for_each(|account| {
				let _ = pallet_balances::Pallet::<T>::transfer(
					&task_account_id,
					&account,
					shard_member_reward,
					ExistenceRequirement::AllowDeath,
				);
			});
			// payout write signer reward and cleanup storage
			SignerPayout::<T>::drain_prefix(task_id).for_each(|(account, amount)| {
				let _ = pallet_balances::Pallet::<T>::transfer(
					&task_account_id,
					&account,
					amount,
					ExistenceRequirement::AllowDeath,
				);
			});
		}
		fn get_gmp_hash(
			task_id: TaskId,
			shard_id: ShardId,
			chain_id: u64,
		) -> Result<Vec<u8>, sp_runtime::DispatchError> {
			let task_descriptor = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			let tss_public_key =
				T::Shards::tss_public_key(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let network = T::Shards::shard_network(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let gateway_contract =
				Gateway::<T>::get(network).ok_or(Error::<T>::GatewayNotRegistered)?;

			let gmp_params = GmpParams {
				chain_id,
				tss_public_key,
				gateway_contract: gateway_contract.into(),
			};

			match task_descriptor.function {
				Function::SendMessage {
					address,
					payload,
					salt,
					gas_limit,
				} => Ok(Message::gmp(chain_id, address, payload, salt, gas_limit)
					.to_eip712_bytes(&gmp_params)
					.into()),
				Function::RegisterShard { shard_id } => {
					let tss_public_key =
						T::Shards::tss_public_key(shard_id).ok_or(Error::<T>::UnknownShard)?;
					Ok(Message::update_keys([], [tss_public_key])
						.to_eip712_bytes(&gmp_params)
						.into())
				},
				Function::UnregisterShard { shard_id } => {
					let tss_public_key =
						T::Shards::tss_public_key(shard_id).ok_or(Error::<T>::UnknownShard)?;
					Ok(Message::update_keys([tss_public_key], [])
						.to_eip712_bytes(&gmp_params)
						.into())
				},
				_ => Err(Error::<T>::InvalidTaskFunction.into()),
			}
		}
	}

	impl<T: Config> TasksInterface for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: NetworkId) {
			NetworkShards::<T>::insert(network, shard_id, ());
			Self::start_task(
				TaskDescriptorParams::new(
					network,
					Function::RegisterShard { shard_id },
					T::Shards::shard_members(shard_id).len() as u32,
				),
				TaskFunder::Shard(shard_id),
			)
			.unwrap();
		}

		fn shard_offline(shard_id: ShardId, network: NetworkId) {
			NetworkShards::<T>::remove(network, shard_id);
			ShardTasks::<T>::drain_prefix(shard_id).for_each(|(task_id, _)| {
				TaskShard::<T>::remove(task_id);
				if Self::is_runnable(task_id) {
					UnassignedTasks::<T>::insert(network, task_id, ());
				}
			});
			if ShardRegistered::<T>::take(shard_id).is_some() {
				Self::start_task(
					TaskDescriptorParams::new(
						network,
						Function::UnregisterShard { shard_id },
						T::Shards::shard_members(shard_id).len() as u32,
					),
					TaskFunder::Shard(shard_id),
				)
				.unwrap();
			} else {
				Self::schedule_tasks(network);
			}
		}
	}
}
