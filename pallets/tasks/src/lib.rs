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
		AccountId, Balance, DepreciationRate, ElectionsInterface, Function, GmpParams, Message,
		Msg, NetworkEvents, NetworkId, Payload, PublicKey, RewardConfig, ShardId, ShardsInterface,
		TaskDescriptor, TaskDescriptorParams, TaskExecution, TaskFunder, TaskId, TaskPhase,
		TaskResult, TasksInterface, TransferStake, TssSignature,
	};

	pub trait WeightInfo {
		fn create_task(input_length: u32) -> Weight;
		fn submit_result(input_length: u32) -> Weight;
		fn submit_hash() -> Weight;
		fn submit_signature() -> Weight;
		fn register_gateway() -> Weight;
		fn set_read_task_reward() -> Weight;
		fn set_write_task_reward() -> Weight;
		fn set_send_message_task_reward() -> Weight;
	}

	impl WeightInfo for () {
		fn create_task(_: u32) -> Weight {
			Weight::default()
		}

		fn submit_result(_: u32) -> Weight {
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
		type Elections: ElectionsInterface;
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
		type SignPhaseTimeout: Get<BlockNumberFor<Self>>;
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
	pub type TaskPhaseState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskPhase, ValueQuery>;

	#[pallet::storage]
	pub type TaskSignature<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TssSignature, OptionQuery>;

	#[pallet::storage]
	pub type TaskSigner<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, PublicKey, OptionQuery>;

	#[pallet::storage]
	pub type TaskHash<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, [u8; 32], OptionQuery>;

	#[pallet::storage]
	pub type PhaseStart<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TaskId,
		Blake2_128Concat,
		TaskPhase,
		BlockNumberFor<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type TaskOutput<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskResult, OptionQuery>;

	#[pallet::storage]
	pub type ShardRegistered<T: Config> = StorageMap<_, Blake2_128Concat, ShardId, (), OptionQuery>;

	#[pallet::storage]
	pub type Gateway<T: Config> = StorageMap<_, Blake2_128Concat, NetworkId, [u8; 20], OptionQuery>;

	#[pallet::storage]
	pub type RecvTasks<T: Config> = StorageMap<_, Blake2_128Concat, NetworkId, u64, OptionQuery>;
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

	#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, TypeInfo)]
	pub enum UnassignedReason {
		NoShardOnline,
		NoShardWithRequestedMembers,
		NoRegisteredShard,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Task succeeded with result
		TaskResult(TaskId, TaskResult),
		/// Gateway registered on network
		GatewayRegistered(NetworkId, [u8; 20], u64),
		/// Gateway contract locked for network
		GatewayLocked(NetworkId),
		/// Read task reward set for network
		ReadTaskRewardSet(NetworkId, BalanceOf<T>),
		/// Write task reward set for network
		WriteTaskRewardSet(NetworkId, BalanceOf<T>),
		/// Send message task reward set for network
		SendMessageTaskRewardSet(NetworkId, BalanceOf<T>),
		/// Task was assigned.
		TaskAssigned(TaskId, ShardId),
		/// Task was not assigned.
		TaskUnassigned(TaskId, UnassignedReason),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Task
		UnknownTask,
		/// Unknown Shard
		UnknownShard,
		/// Invalid Signature
		InvalidSignature,
		/// Signature Verification Failed
		SignatureVerificationFailed,
		/// Invalid Owner
		InvalidOwner,
		/// Invalid task function
		InvalidTaskFunction,
		/// Not sign phase
		NotSignPhase,
		/// Not write phase
		NotWritePhase,
		/// Not read phase
		NotReadPhase,
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
		/// Shard for task must be online at task creation
		MatchingShardNotOnline,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::create_task(schedule.function.get_input_length()))]
		pub fn create_task(origin: OriginFor<T>, schedule: TaskDescriptorParams) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				T::Shards::matching_shard_online(schedule.network, schedule.shard_size),
				Error::<T>::MatchingShardNotOnline
			);
			Self::start_task(schedule, TaskFunder::Account(who))?;
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_result(result.payload.get_input_length()))]
		pub fn submit_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			result: TaskResult,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			if TaskOutput::<T>::get(task_id).is_some() {
				return Ok(());
			}
			ensure!(TaskPhaseState::<T>::get(task_id) == TaskPhase::Read, Error::<T>::NotReadPhase);
			let shard_id = TaskShard::<T>::get(task_id).ok_or(Error::<T>::UnassignedTask)?;
			ensure!(result.shard_id == shard_id, Error::<T>::InvalidOwner);
			let bytes = result.payload.bytes(task_id);
			Self::validate_signature(result.shard_id, &bytes, result.signature)?;
			TaskOutput::<T>::insert(task_id, result.clone());
			if let Some(shard_id) = TaskShard::<T>::take(task_id) {
				ShardTasks::<T>::remove(shard_id, task_id);
			}
			Self::payout_task_rewards(task_id, result.shard_id, task.function.initial_phase());
			if let Function::RegisterShard { shard_id } = task.function {
				ShardRegistered::<T>::insert(shard_id, ());
			}
			if let Payload::Gmp(msgs) = &result.payload {
				for msg in msgs {
					Self::send_message(result.shard_id, msg.clone());
				}
			}
			Self::deposit_event(Event::TaskResult(task_id, result));
			Ok(())
		}

		/// Submit Task Hash
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_hash())]
		pub fn submit_hash(
			origin: OriginFor<T>,
			task_id: TaskId,
			hash: [u8; 32],
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			ensure!(
				TaskPhaseState::<T>::get(task_id) == TaskPhase::Write,
				Error::<T>::NotWritePhase
			);
			let expected_signer = TaskSigner::<T>::get(task_id);
			ensure!(
				Some(signer.clone()) == expected_signer.map(|s| s.into_account()),
				Error::<T>::InvalidSigner
			);
			let shard_id = TaskShard::<T>::get(task_id).ok_or(Error::<T>::UnassignedTask)?;
			Self::snapshot_write_reward(task_id, signer);
			TaskHash::<T>::insert(task_id, hash);
			Self::start_phase(shard_id, task_id, TaskPhase::Read);
			Ok(())
		}

		/// Submit Signature
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_signature())]
		pub fn submit_signature(
			origin: OriginFor<T>,
			task_id: TaskId,
			signature: TssSignature,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(TaskSignature::<T>::get(task_id).is_none(), Error::<T>::TaskSigned);
			ensure!(Tasks::<T>::get(task_id).is_some(), Error::<T>::UnknownTask);
			ensure!(TaskPhaseState::<T>::get(task_id) == TaskPhase::Sign, Error::<T>::NotSignPhase);
			let Some(shard_id) = TaskShard::<T>::get(task_id) else {
				return Err(Error::<T>::UnassignedTask.into());
			};
			let bytes = Self::get_gmp_hash(task_id, shard_id)?;
			Self::validate_signature(shard_id, &bytes, signature)?;
			Self::start_phase(shard_id, task_id, TaskPhase::Write);
			TaskSignature::<T>::insert(task_id, signature);
			Ok(())
		}

		/// Register gateway
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::register_gateway())]
		pub fn register_gateway(
			origin: OriginFor<T>,
			bootstrap: ShardId,
			address: [u8; 20],
			block_height: u64,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(T::Shards::is_shard_online(bootstrap), Error::<T>::BootstrapShardMustBeOnline);
			let network = T::Shards::shard_network(bootstrap).ok_or(Error::<T>::UnknownShard)?;
			// reset all previous GMP tasks to sign phase once gateway is registered
			Tasks::<T>::iter()
				.filter(|(t, n)| {
					n.network == network
						&& matches!(n.function, Function::SendMessage { .. })
						&& !matches!(TaskPhaseState::<T>::get(t), TaskPhase::Read)
				})
				.for_each(|(t, _)| {
					// reset to sign phase
					TaskPhaseState::<T>::insert(t, TaskPhase::Sign);
				});
			for (shard_id, _) in NetworkShards::<T>::iter_prefix(network) {
				ShardRegistered::<T>::remove(shard_id);
			}
			ShardRegistered::<T>::insert(bootstrap, ());
			for (shard_id, _) in NetworkShards::<T>::iter_prefix(network) {
				if shard_id != bootstrap {
					Self::register_shard(shard_id, network);
				}
			}
			Gateway::<T>::insert(network, address);
			RecvTasks::<T>::insert(network, block_height);
			Self::schedule_tasks(network);
			Self::deposit_event(Event::GatewayRegistered(network, address, block_height));
			Ok(())
		}

		/// Set read task reward
		#[pallet::call_index(5)]
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
		#[pallet::call_index(6)]
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
		#[pallet::call_index(7)]
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
		fn on_initialize(current: BlockNumberFor<T>) -> Weight {
			let mut writes = 0;
			TaskShard::<T>::iter().for_each(|(task_id, shard_id)| {
				let phase = TaskPhaseState::<T>::get(task_id);
				let start = PhaseStart::<T>::get(task_id, phase);
				let timeout = match phase {
					TaskPhase::Sign => T::SignPhaseTimeout::get(),
					TaskPhase::Write => T::WritePhaseTimeout::get(),
					TaskPhase::Read => T::ReadPhaseTimeout::get(),
				};
				if current.saturating_sub(start) >= timeout {
					if phase == TaskPhase::Write {
						Self::start_phase(shard_id, task_id, phase);
					} else {
						Self::schedule_task(task_id);
					}
					writes += 3;
				}
			});
			T::DbWeight::get().writes(writes)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_task_signature(task: TaskId) -> Option<TssSignature> {
			TaskSignature::<T>::get(task)
		}

		pub fn get_task_signer(task: TaskId) -> Option<PublicKey> {
			TaskSigner::<T>::get(task)
		}

		pub fn get_task_hash(task: TaskId) -> Option<[u8; 32]> {
			TaskHash::<T>::get(task)
		}

		pub fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution> {
			ShardTasks::<T>::iter_prefix(shard_id)
				.map(|(task_id, _)| TaskExecution::new(task_id, TaskPhaseState::<T>::get(task_id)))
				.collect()
		}

		pub fn get_task(task_id: TaskId) -> Option<TaskDescriptor> {
			Tasks::<T>::get(task_id)
		}

		pub fn get_gateway(network: NetworkId) -> Option<[u8; 20]> {
			Gateway::<T>::get(network)
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
	}

	impl<T: Config> Pallet<T> {
		/// The account ID containing the funds for a task.
		fn task_account(task_id: TaskId) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(task_id)
		}

		fn recv_messages(network_id: NetworkId, block_height: u64) {
			let mut task = TaskDescriptorParams::new(
				network_id,
				Function::ReadMessages,
				T::Elections::default_shard_size(),
			);
			task.start = block_height;
			Self::start_task(task, TaskFunder::Inflation).expect("task funded through inflation");
		}

		fn register_shard(shard_id: ShardId, network_id: NetworkId) {
			Self::start_task(
				TaskDescriptorParams::new(
					network_id,
					Function::RegisterShard { shard_id },
					T::Shards::shard_members(shard_id).len() as _,
				),
				TaskFunder::Inflation,
			)
			.expect("task funded through inflation");
		}

		fn send_message(shard_id: ShardId, msg: Msg) {
			Self::start_task(
				TaskDescriptorParams::new(
					msg.dest_network,
					Function::SendMessage { msg },
					T::Shards::shard_members(shard_id).len() as _,
				),
				TaskFunder::Inflation,
			)
			.expect("task funded through inflation");
		}

		/// Start task
		fn start_task(schedule: TaskDescriptorParams, who: TaskFunder) -> DispatchResult {
			let task_id = TaskIdCounter::<T>::get();
			let phase = schedule.function.initial_phase();
			let (read_task_reward, write_task_reward, send_message_reward) = (
				T::BaseReadReward::get() + NetworkReadReward::<T>::get(schedule.network),
				T::BaseWriteReward::get() + NetworkWriteReward::<T>::get(schedule.network),
				T::BaseSendMessageReward::get()
					+ NetworkSendMessageReward::<T>::get(schedule.network),
			);
			let mut required_funds = read_task_reward.saturating_mul(schedule.shard_size.into());
			if phase == TaskPhase::Write || phase == TaskPhase::Sign {
				required_funds = required_funds.saturating_add(write_task_reward);
			}
			if phase == TaskPhase::Sign {
				required_funds = required_funds
					.saturating_add(send_message_reward.saturating_mul(schedule.shard_size.into()));
			}
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
				TaskFunder::Inflation => {
					pallet_balances::Pallet::<T>::resolve_creating(
						&Self::task_account(task_id),
						pallet_balances::Pallet::<T>::issue(required_funds),
					);
					None
				},
			};
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
			TaskPhaseState::<T>::insert(task_id, phase);
			TaskIdCounter::<T>::put(task_id.saturating_plus_one());
			UnassignedTasks::<T>::insert(schedule.network, task_id, ());
			Self::deposit_event(Event::TaskCreated(task_id));
			Self::schedule_task(task_id);
			Ok(())
		}

		fn start_phase(shard_id: ShardId, task_id: TaskId, phase: TaskPhase) {
			let block = frame_system::Pallet::<T>::block_number();
			TaskPhaseState::<T>::insert(task_id, phase);
			PhaseStart::<T>::insert(task_id, phase, block);
			if phase == TaskPhase::Write {
				TaskSigner::<T>::insert(task_id, T::Shards::next_signer(shard_id));
			}
		}

		fn validate_signature(
			shard_id: ShardId,
			data: &[u8],
			signature: TssSignature,
		) -> DispatchResult {
			let public_key = T::Shards::tss_public_key(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let signature = schnorr_evm::Signature::from_bytes(signature)
				.map_err(|_| Error::<T>::InvalidSignature)?;
			let schnorr_public_key = schnorr_evm::VerifyingKey::from_bytes(public_key)
				.map_err(|_| Error::<T>::UnknownShard)?;
			schnorr_public_key
				.verify(data, &signature)
				.map_err(|_| Error::<T>::SignatureVerificationFailed)?;
			Ok(())
		}

		fn shard_task_count(shard_id: ShardId) -> usize {
			ShardTasks::<T>::iter_prefix(shard_id).count()
		}

		fn schedule_tasks(network: NetworkId) {
			for (task_id, _) in UnassignedTasks::<T>::iter_prefix(network) {
				Self::schedule_task(task_id);
			}
		}

		fn schedule_task(task_id: TaskId) {
			let Some(task) = Tasks::<T>::get(task_id) else {
				// this branch should never be hit, maybe turn this into expect
				return;
			};
			let Some(shard_id) = Self::select_shard(task.network, task_id, task.shard_size) else {
				// on gmp task sometimes returns none and it stops every other schedule
				return;
			};
			if let Some(old_shard_id) = TaskShard::<T>::get(task_id) {
				ShardTasks::<T>::remove(old_shard_id, task_id);
			}
			UnassignedTasks::<T>::remove(task.network, task_id);
			ShardTasks::<T>::insert(shard_id, task_id, ());
			TaskShard::<T>::insert(task_id, shard_id);
			Self::start_phase(shard_id, task_id, TaskPhaseState::<T>::get(task_id));
		}

		/// Select shard for task assignment
		/// Selects the shard for the input Network with the least number of tasks
		/// assigned to it.
		/// Excludes selecting the `old` shard_id optional input if it is passed
		/// for task reassignment purposes.
		fn select_shard(network: NetworkId, task_id: TaskId, shard_size: u16) -> Option<ShardId> {
			let mut reason = UnassignedReason::NoShardOnline;
			let mut selected = None;
			let mut selected_tasks = usize::MAX;
			let mut plan_b = None;
			let mut plan_b_tasks = usize::MAX;
			for (shard_id, _) in NetworkShards::<T>::iter_prefix(network) {
				if !T::Shards::is_shard_online(shard_id) {
					continue;
				}
				reason = core::cmp::max(reason, UnassignedReason::NoShardWithRequestedMembers);
				if T::Shards::shard_members(shard_id).len() != shard_size as usize {
					continue;
				}
				reason = core::cmp::max(reason, UnassignedReason::NoRegisteredShard);
				if TaskPhaseState::<T>::get(task_id) == TaskPhase::Sign {
					if Gateway::<T>::get(network).is_none() {
						continue;
					}
					if ShardRegistered::<T>::get(shard_id).is_none() {
						continue;
					}
				}

				let tasks = Self::shard_task_count(shard_id);
				if tasks < selected_tasks {
					selected = Some(shard_id);
					selected_tasks = tasks;
				} else if tasks < plan_b_tasks {
					plan_b = Some(shard_id);
					plan_b_tasks = tasks;
				}
			}

			if let Some(shard_id) = &mut selected {
				let old = TaskShard::<T>::get(task_id);
				if let (Some(previous_shard), Some(plan_b)) = (old, plan_b) {
					if previous_shard == *shard_id {
						*shard_id = plan_b;
					}
				}
				Self::deposit_event(Event::TaskAssigned(task_id, *shard_id));
			} else {
				Self::deposit_event(Event::TaskUnassigned(task_id, reason));
			}
			selected
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
					PhaseStart::<T>::take(task_id, TaskPhase::Write),
					write_task_reward,
					depreciation_rate,
				),
			);
		}

		fn payout_task_rewards(task_id: TaskId, shard_id: ShardId, phase: TaskPhase) {
			let task_account_id = Self::task_account(task_id);
			let start = PhaseStart::<T>::take(task_id, TaskPhase::Read);
			let Some(RewardConfig {
				read_task_reward,
				send_message_reward,
				depreciation_rate,
				..
			}) = TaskRewardConfig::<T>::take(task_id)
			else {
				return;
			};
			let mut shard_member_reward =
				Self::apply_depreciation(start, read_task_reward, depreciation_rate.clone());
			if phase == TaskPhase::Sign {
				let send_msg_reward =
					Self::apply_depreciation(start, send_message_reward, depreciation_rate);
				shard_member_reward = shard_member_reward.saturating_add(send_msg_reward);
			}
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
		) -> Result<Vec<u8>, sp_runtime::DispatchError> {
			let task_descriptor = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			let tss_public_key =
				T::Shards::tss_public_key(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let network_id = T::Shards::shard_network(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let gateway_contract =
				Gateway::<T>::get(network_id).ok_or(Error::<T>::GatewayNotRegistered)?;

			let gmp_params = GmpParams {
				network_id,
				tss_public_key,
				gateway_contract: gateway_contract.into(),
			};

			match task_descriptor.function {
				Function::SendMessage { msg } => {
					Ok(Message::gmp(msg).to_eip712_bytes(&gmp_params).into())
				},
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
			if Gateway::<T>::get(network).is_some() {
				Self::register_shard(shard_id, network);
			}
			Self::schedule_tasks(network);
		}

		fn shard_offline(shard_id: ShardId, network: NetworkId) {
			NetworkShards::<T>::remove(network, shard_id);
			ShardTasks::<T>::drain_prefix(shard_id).for_each(|(task_id, _)| {
				TaskShard::<T>::remove(task_id);
				UnassignedTasks::<T>::insert(network, task_id, ());
			});
			let less_than_one_shard_online =
				NetworkShards::<T>::iter_prefix(network).next().is_none();
			if less_than_one_shard_online {
				ShardRegistered::<T>::remove(shard_id);
				Gateway::<T>::remove(network);
				Tasks::<T>::iter().filter(|(_, n)| n.network == network).for_each(|(t, _)| {
					// Task failed because gateway locked
					Tasks::<T>::remove(t);
					TaskPhaseState::<T>::remove(t);
					TaskOutput::<T>::insert(
						t,
						TaskResult {
							shard_id,
							payload: Payload::Error("Gateway locked".into()),
							signature: [0u8; 64],
						},
					);
				});
				Self::deposit_event(Event::GatewayLocked(network));
				return;
			}
			if ShardRegistered::<T>::take(shard_id).is_some() {
				Self::start_task(
					TaskDescriptorParams::new(
						network,
						Function::UnregisterShard { shard_id },
						T::Shards::shard_members(shard_id).len() as _,
					),
					TaskFunder::Inflation,
				)
				.expect("task funded through inflation");
			}
			Self::schedule_tasks(network);
		}
	}

	impl<T: Config> NetworkEvents for Pallet<T> {
		fn block_height_changed(network_id: NetworkId, block_height: u64) {
			if let Some(current) = RecvTasks::<T>::get(network_id) {
				let next = block_height + 10;
				if current < next {
					for block_height in current..next {
						Self::recv_messages(network_id, block_height);
					}
					RecvTasks::<T>::insert(network_id, next);
				}
			}
		}
	}
}
