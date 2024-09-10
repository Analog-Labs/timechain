#![cfg_attr(not(feature = "std"), no_std)]
//! # Timechain Task Pallet
//!
//! This chart shows all the extrinsics and events of the task pallet. It
//! categorizes the extrinsics into two types: those that can be called by
//! root or council, and those that can be called by any user. The root or
//! council extrinsics are related to administrative tasks, while user
//! extrinsics are related to task operations and submissions.
//!
#![doc = simple_mermaid::mermaid!("../docs/tasks_extrinsics.mmd")]
//!
//! ## **Task Pallet Lifecycle and Operations**
//!
// #![doc = simple_mermaid::mermaid!("../docs/tasks_tb.mmd")]
//!
//! This flowchart outlines the lifecycle of a task in the task pallet. It
//! starts with task creation and checks if the shard is online. If successful,
//! the task is assigned to a shard and progresses through various phases
//! (sign, write, read). Upon completion, rewards are paid out. Tasks can also
//! be reset, and error handling includes retrying or canceling tasks.
//!
#![doc = simple_mermaid::mermaid!("../docs/tasks_lr.mmd")]
//!
//! ## **Unregister Gateways**
//! This flowchart illustrates the process for unregistering gateways in the
//! task pallet. It ensures that only a root user can perform this action. The
//! process involves clearing a specified number of gateways, all registered
//! shards, and filtering tasks to determine their status and handle them
//! appropriately. Errors during the process are logged and returned.
//!
#![doc = simple_mermaid::mermaid!("../docs/unregister_gateways.mmd")]
//!
//! ## **Reset Tasks**
//! This flowchart shows the process for resetting tasks in the task pallet. It
//! ensures that only a root user can initiate the reset. The reset process
//! includes iterating over unassigned tasks and tasks associated with
//! specific shards, resetting their phase state, and adding them back to
//! unassigned tasks if necessary. The iteration stops once the maximum
//! number of tasks to be reset is reached.
#![doc = simple_mermaid::mermaid!("../docs/reset_tasks.mmd")]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub use pallet::*;
#[cfg(test)]
mod mock;
pub mod queue;
#[cfg(test)]
mod tests;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use crate::queue::*;
	use core::num::NonZeroU64;
	use scale_info::prelude::string::String;

	use polkadot_sdk::{
		frame_support, frame_system, pallet_balances, pallet_treasury, sp_runtime, sp_std,
	};

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
	use sp_std::boxed::Box;
	use sp_std::vec;
	use sp_std::vec::Vec;

	use time_primitives::{
		AccountId, Balance, DepreciationRate, ElectionsInterface, Function, GmpParams, Message,
		Msg, NetworkId, Payload, PublicKey, RewardConfig, ShardId, ShardsInterface, TaskDescriptor,
		TaskDescriptorParams, TaskExecution, TaskFunder, TaskId, TaskIndex, TaskPhase, TaskResult,
		TasksInterface, TransferStake, TssSignature,
	};

	/// Trait to define the weights for various extrinsics in the pallet.
	pub trait WeightInfo {
		fn create_task(input_length: u32) -> Weight;
		fn submit_result() -> Weight;
		fn submit_hash() -> Weight;
		fn submit_signature() -> Weight;
		fn register_gateway() -> Weight;
		fn set_read_task_reward() -> Weight;
		fn set_write_task_reward() -> Weight;
		fn set_send_message_task_reward() -> Weight;
		fn sudo_cancel_task() -> Weight;
		fn sudo_cancel_tasks(n: u32) -> Weight;
		fn reset_tasks(n: u32) -> Weight;
		fn set_shard_task_limit() -> Weight;
		fn unregister_gateways(n: u32) -> Weight;
		fn set_batch_size() -> Weight;
	}

	impl WeightInfo for () {
		fn create_task(_: u32) -> Weight {
			Weight::default()
		}

		fn submit_result() -> Weight {
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

		fn sudo_cancel_task() -> Weight {
			Weight::default()
		}

		fn sudo_cancel_tasks(_: u32) -> Weight {
			Weight::default()
		}

		fn reset_tasks(_: u32) -> Weight {
			Weight::default()
		}

		fn set_shard_task_limit() -> Weight {
			Weight::default()
		}

		fn unregister_gateways(_: u32) -> Weight {
			Weight::default()
		}

		fn set_batch_size() -> Weight {
			Weight::default()
		}
	}

	type BalanceOf<T> = <T as pallet_balances::Config>::Balance;

	/// A constant defining the batch size for task processing, set to 32.
	const BATCH_SIZE: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(32) };

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<AccountId = AccountId>
		+ pallet_balances::Config<Balance = Balance>
		+ pallet_treasury::Config
	{
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
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

	/// Double map storage for unassigned system tasks.
	#[pallet::storage]
	pub type UnassignedSystemTasks<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		NetworkId,
		Blake2_128Concat,
		TaskIndex,
		TaskId,
		OptionQuery,
	>;

	/// Double map storage for unassigned tasks.
	#[pallet::storage]
	pub type UnassignedTasks<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		NetworkId,
		Blake2_128Concat,
		TaskIndex,
		TaskId,
		OptionQuery,
	>;

	/// Map storage for the insert index of unassigned system tasks.
	#[pallet::storage]
	pub type UASystemTasksInsertIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, TaskIndex, OptionQuery>;

	/// Map storage for the remove index of unassigned system tasks.
	#[pallet::storage]
	pub type UASystemTasksRemoveIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, TaskIndex, OptionQuery>;

	/// Map storage for the insert index of unassigned tasks.
	#[pallet::storage]
	pub type UATasksInsertIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, TaskIndex, OptionQuery>;

	/// Map storage for the remove index of unassigned tasks.
	#[pallet::storage]
	pub type UATasksRemoveIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, TaskIndex, OptionQuery>;

	/// Map storage for task index by task ID.
	#[pallet::storage]
	pub type UATaskIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskIndex, OptionQuery>;

	/// Map storage for shard task limits.
	#[pallet::storage]
	#[pallet::getter(fn shard_task_limit)]
	pub type ShardTaskLimit<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u32, OptionQuery>;

	/// Double map storage for tasks by shard.
	#[pallet::storage]
	pub type ShardTasks<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TaskId, (), OptionQuery>;

	/// Map storage for task shard by task ID.
	#[pallet::storage]
	#[pallet::getter(fn task_shard)]
	pub type TaskShard<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, ShardId, OptionQuery>;

	/// Double map storage for network shards.
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

	///  Map storage for network batch sizes.
	#[pallet::storage]
	pub type NetworkBatchSize<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u64, OptionQuery>;

	/// Map storage for network offsets.
	#[pallet::storage]
	pub type NetworkOffset<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, u64, OptionQuery>;

	/// Storage for task ID counter.
	#[pallet::storage]
	pub type TaskIdCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Map storage for tasks.
	#[pallet::storage]
	#[pallet::getter(fn tasks)]
	pub type Tasks<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskDescriptor, OptionQuery>;

	/// Map storage for task phase states.
	#[pallet::storage]
	pub type TaskPhaseState<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskPhase, ValueQuery>;

	/// Map storage for task signatures.
	#[pallet::storage]
	pub type TaskSignature<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TssSignature, OptionQuery>;

	/// Map storage for task signers.
	#[pallet::storage]
	pub type TaskSigner<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, PublicKey, OptionQuery>;

	/// Map storage for task hashes.
	#[pallet::storage]
	pub type TaskHash<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, [u8; 32], OptionQuery>;

	/// Double map storage for phase start times.
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

	/// Map storage for task outputs.
	#[pallet::storage]
	pub type TaskOutput<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, TaskResult, OptionQuery>;

	/// Map storage for registered shards.
	#[pallet::storage]
	pub type ShardRegistered<T: Config> = StorageMap<_, Blake2_128Concat, ShardId, (), OptionQuery>;

	/// Map storage for network gateways.
	#[pallet::storage]
	pub type Gateway<T: Config> = StorageMap<_, Blake2_128Concat, NetworkId, [u8; 20], OptionQuery>;

	///  Map storage for received tasks.
	#[pallet::storage]
	pub type RecvTasks<T: Config> = StorageMap<_, Blake2_128Concat, NetworkId, u64, OptionQuery>;

	///  Map storage for network read rewards.
	#[pallet::storage]
	#[pallet::getter(fn network_read_reward)]
	pub type NetworkReadReward<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, BalanceOf<T>, ValueQuery>;

	/// Map storage for network write rewards.
	#[pallet::storage]
	#[pallet::getter(fn network_write_reward)]
	pub type NetworkWriteReward<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, BalanceOf<T>, ValueQuery>;

	/// Map storage for network send message rewards.
	#[pallet::storage]
	#[pallet::getter(fn network_send_message_reward)]
	pub type NetworkSendMessageReward<T: Config> =
		StorageMap<_, Blake2_128Concat, NetworkId, BalanceOf<T>, ValueQuery>;

	/// Map storage for task reward configurations.
	#[pallet::storage]
	#[pallet::getter(fn task_reward_config)]
	pub type TaskRewardConfig<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		TaskId,
		RewardConfig<BalanceOf<T>, BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// Double map storage for signer payouts.
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

	/// Map storage for message tasks.
	#[pallet::storage]
	pub type MessageTask<T: Config> =
		StorageMap<_, Blake2_128Concat, [u8; 32], (TaskId, TaskId), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Task succeeded with result
		TaskResult(TaskId, TaskResult),
		/// Gateway registered on network
		GatewayRegistered(NetworkId, [u8; 20], u64),
		/// Read task reward set for network
		ReadTaskRewardSet(NetworkId, BalanceOf<T>),
		/// Write task reward set for network
		WriteTaskRewardSet(NetworkId, BalanceOf<T>),
		/// Send message task reward set for network
		SendMessageTaskRewardSet(NetworkId, BalanceOf<T>),
		/// Set the maximum number of assigned tasks for all shards on the network
		ShardTaskLimitSet(NetworkId, u32),
		/// Set the network batch size
		BatchSizeSet(NetworkId, u64, u64),
		/// Insufficient Treasury Balance to create RegisterShard task
		InsufficientTreasuryBalanceToRegisterShard(ShardId),
		/// Insufficient Treasury Balance to create UnRegisterShard task
		InsufficientTreasuryBalanceToUnRegisterShard(ShardId),
		/// Insufficient Treasury Balance To Create SendMessage task as an effect of calling `submit_result`
		InsufficientTreasuryBalanceToSendMessage(Msg),
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
		/// Insufficient balance in Treasury to fund register_gateway Recv Tasks
		InsufficientTreasuryBalanceToRecvTasks,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Allows users to create tasks on the blockchain.
		///
		/// # Flow
		///    1. Ensure the transaction is signed.
		///    2. Verify that a matching shard is online.
		///    3. Start the task with the provided parameters.
		///    4. Return success or failure.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::create_task(schedule.function.get_input_length()))]
		pub fn create_task(origin: OriginFor<T>, schedule: TaskDescriptorParams) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				T::Shards::matching_shard_online(schedule.network, schedule.shard_size),
				Error::<T>::MatchingShardNotOnline
			);
			Self::start_task(schedule, TaskFunder::Treasury)?;
			Ok(())
		}

		/// Allows users to submit results for a task.
		///
		/// # Flow
		///    1. Ensure the transaction is signed.
		///    2. Verify the existence and state of the task.
		///    3. Validate the shard ownership and the result's signature.
		///    4. Mark the task as completed and process any associated actions.
		///    5. Emit relevant events and schedule new tasks.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_result())]
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
			match &result.payload {
				Payload::Gmp(msgs) => {
					if let Some(block_height) = RecvTasks::<T>::get(task.network) {
						let batch_size = NetworkBatchSize::<T>::get(task.network)
							.and_then(NonZeroU64::new)
							.unwrap_or(BATCH_SIZE);
						if let Some(next_block_height) = block_height.checked_add(batch_size.into())
						{
							Self::recv_messages(task.network, next_block_height, batch_size)?;
						}
					}
					for msg in msgs {
						if let Ok(send_task_id) = Self::send_message(result.shard_id, msg.clone()) {
							MessageTask::<T>::insert(msg.salt, (task_id, send_task_id));
						} else {
							Self::deposit_event(Event::InsufficientTreasuryBalanceToSendMessage(
								msg.clone(),
							));
						}
					}
				},
				Payload::Error(_) => {
					if let Function::ReadMessages { batch_size } = task.function {
						if let Some(block_height) = RecvTasks::<T>::get(task.network) {
							Self::recv_messages(task.network, block_height, batch_size)?;
						}
					}
				},
				_ => {},
			}
			Self::finish_task(task_id, result.clone());
			Self::payout_task_rewards(task_id, result.shard_id, task.function.initial_phase());
			if let Function::RegisterShard { shard_id } = task.function {
				ShardRegistered::<T>::insert(shard_id, ());
			}
			Self::deposit_event(Event::TaskResult(task_id, result));
			Ok(())
		}

		/// Allows users to submit a hash for a task.
		///
		/// # Flow
		///    1. Ensure the transaction is signed.
		///    2. Verify the existence and state of the task.
		///    3. Validate the signer of the transaction.
		///    4. Handle reward snapshotting for the task writer.
		///    5. Insert the hash and transition the task to the next phase if successful or finish the task with an error payload if there is an error.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_hash())]
		pub fn submit_hash(
			origin: OriginFor<T>,
			task_id: TaskId,
			hash: Result<[u8; 32], String>,
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
			match hash {
				Ok(hash) => {
					TaskHash::<T>::insert(task_id, hash);
					Self::start_phase(shard_id, task_id, TaskPhase::Read);
				},
				Err(err) => {
					let result = TaskResult {
						shard_id,
						payload: Payload::Error(err),
						signature: [0; 64],
					};
					Self::finish_task(task_id, result.clone());
					Self::deposit_event(Event::TaskResult(task_id, result));
				},
			}
			Ok(())
		}

		/// Handles the submission and validation of a signature for a specific task.
		/// # Flow
		///    1. Validate the transaction origin is signed.
		///    2. Ensure no existing signature exists for the task.
		///    3. Verify the task exists and is in the appropriate `Sign` phase.
		///    4. Retrieve the shard ID associated with the task and ensure it's assigned.
		///    5. Compute the hash for the task.
		///    6. Validate the provided signature against the computed hash.
		///    7. Advance the task phase to `Write`.
		///    8. Store the validated signature for the task in the pallet's storage.
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

		/// Registers a new gateway for a network.
		///
		/// # Flow
		///    1. Ensure the origin of the transaction is a root user.
		///    2. Check if the bootstrap shard is online.
		///    3. Retrieve the network associated with the bootstrap shard.
		///    4. Unregister all shards in the network.
		///    5. Register the bootstrap shard.
		///    6. Re-register all other shards in the network.
		///    7. Check if a gateway was already registered for the network.
		///    8. Register the new gateway address.
		///    9. Emit a [`Event::GatewayRegistered`] event.
		///   10. If no gateway was previously registered, calculate the next block height and batch size for message reception.
		///   11. Receive messages for the network.
		///   12. Schedule tasks for the network.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::register_gateway())]
		pub fn register_gateway(
			origin: OriginFor<T>,
			bootstrap: ShardId,
			address: [u8; 20],
			block_height: u64,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			ensure!(T::Shards::is_shard_online(bootstrap), Error::<T>::BootstrapShardMustBeOnline);
			let network = T::Shards::shard_network(bootstrap).ok_or(Error::<T>::UnknownShard)?;
			for (shard_id, _) in NetworkShards::<T>::iter_prefix(network) {
				Self::unregister_shard(shard_id, network);
			}
			ShardRegistered::<T>::insert(bootstrap, ());
			for (shard_id, _) in NetworkShards::<T>::iter_prefix(network) {
				if shard_id != bootstrap {
					Self::register_shard(shard_id, network);
				}
			}
			let gateway_changed = Gateway::<T>::get(network).is_some();
			Gateway::<T>::insert(network, address);
			Self::deposit_event(Event::GatewayRegistered(network, address, block_height));
			if !gateway_changed {
				let network_batch_size = NetworkBatchSize::<T>::get(network)
					.and_then(NonZeroU64::new)
					.unwrap_or(BATCH_SIZE);
				let network_offset = NetworkOffset::<T>::get(network).unwrap_or(0);
				let batch_size = NonZeroU64::new(network_batch_size.get() - ((block_height + network_offset) % network_batch_size))
					.expect("x = block_height % BATCH_SIZE ==> x <= BATCH_SIZE - 1 ==> BATCH_SIZE - x >= 1; QED");
				let block_height = block_height + batch_size.get();
				Self::recv_messages(network, block_height, batch_size)?;
			}
			Ok(())
		}

		/// Sets the reward for read tasks.
		///
		/// # Flow
		///    1. Ensure the origin of the transaction is a root user.
		///    2. Insert the new reward amount for the specified network into the [`NetworkReadReward`] storage.
		///    3. Emit an event [`Event::ReadTaskRewardSet`] indicating the read task reward has been set.
		///    4. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::set_read_task_reward())]
		pub fn set_read_task_reward(
			origin: OriginFor<T>,
			network: NetworkId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			NetworkReadReward::<T>::insert(network, amount);
			Self::deposit_event(Event::ReadTaskRewardSet(network, amount));
			Ok(())
		}

		/// Sets the reward for write tasks.
		///
		/// # Flow
		///    1. Ensure the origin of the transaction is a root user.
		///    2. Insert the new reward amount for the specified network into the [`NetworkWriteReward`] storage.
		///    3. Emit an event [`Event::WriteTaskRewardSet`] indicating the write task reward has been set.
		///    4. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::set_write_task_reward())]
		pub fn set_write_task_reward(
			origin: OriginFor<T>,
			network: NetworkId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			NetworkWriteReward::<T>::insert(network, amount);
			Self::deposit_event(Event::WriteTaskRewardSet(network, amount));
			Ok(())
		}

		/// Sets the reward for send message tasks.
		///
		/// # Flow
		///    1. Ensure the origin of the transaction is a root user.
		///    2. Insert the new reward amount for the specified network into the [`NetworkSendMessageReward`] storage.
		///    3. Emit an event [`Event::SendMessageTaskRewardSet`] indicating the send message task reward has been set.
		///    4. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::set_send_message_task_reward())]
		pub fn set_send_message_task_reward(
			origin: OriginFor<T>,
			network: NetworkId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			NetworkSendMessageReward::<T>::insert(network, amount);
			Self::deposit_event(Event::SendMessageTaskRewardSet(network, amount));
			Ok(())
		}

		/// Cancels a specific task.
		///
		/// # Flow
		///    1. Ensure the origin of the transaction is a root user.
		///    2. Check if the task with the specified `task_id` exists.
		///    3. If the task exists, call the `cancel_task` function to handle the cancellation logic.
		///    4. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::sudo_cancel_task())]
		pub fn sudo_cancel_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::UnknownTask)?;
			Self::cancel_task(task_id, task.network);
			Ok(())
		}

		///  Cancels a specified number of tasks.
		/// # Flow
		///    1. Ensure the origin of the transaction is a root user.
		///    2. Iterate over unassigned tasks and cancel them until the specified maximum is reached.
		///    3. If the maximum is not reached, iterate over shard tasks and cancel them until the specified maximum is reached.
		///    4. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::sudo_cancel_tasks(*max))]
		pub fn sudo_cancel_tasks(origin: OriginFor<T>, max: u32) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			// TODO (followup): ensure max <= PalletMax which is set according to our current block size limit
			let mut cancelled = 0;
			for (network, _, task_id) in
				UnassignedTasks::<T>::iter().chain(UnassignedSystemTasks::<T>::iter())
			{
				if cancelled >= max {
					return Ok(());
				}
				Self::cancel_task(task_id, network);
				cancelled = cancelled.saturating_plus_one();
			}
			for (shard_id, task_id, _) in ShardTasks::<T>::iter() {
				if let Some(network) = T::Shards::shard_network(shard_id) {
					if cancelled >= max {
						return Ok(());
					}
					Self::cancel_task(task_id, network);
					cancelled = cancelled.saturating_plus_one();
				}
			}
			Ok(())
		}

		/// Resets a specified number of tasks.
		///
		/// # Flow
		///  1. Ensure the origin of the transaction is a root user.
		///  2. Iterate over [`TaskShard`] storage to remove entries and add them to [`UnassignedTasks`] until the specified maximum is reached.
		///  3. Iterate over [`UnassignedTasks`] storage to reset the task phase state until the specified maximum is reached.
		///  4. Iterate over [`NetworkShards`] storage to schedule tasks for the network and shard.
		///  5. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::reset_tasks(*max))]
		pub fn reset_tasks(origin: OriginFor<T>, max: u32) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let mut to_be_reset = 0u32;
			for (task_id, shard_id) in TaskShard::<T>::drain() {
				ShardTasks::<T>::remove(shard_id, task_id);
				if let Some(task) = Tasks::<T>::get(task_id) {
					if to_be_reset >= max {
						break;
					}
					Self::add_unassigned_task(task.network, task_id);
					to_be_reset = to_be_reset.saturating_plus_one();
				}
			}
			let mut reset = 0u32;
			for (_, _, task_id) in
				UnassignedTasks::<T>::iter().chain(UnassignedSystemTasks::<T>::iter())
			{
				if let Some(task) = Tasks::<T>::get(task_id) {
					if reset >= max {
						break;
					}
					TaskPhaseState::<T>::insert(task_id, task.function.initial_phase());
					reset = reset.saturating_plus_one();
				}
			}
			Ok(())
		}

		///  Sets the task limit for a specific shard.
		/// # Flow
		///    1. Ensure the origin of the transaction is a root user.
		///    2. Insert the new task limit for the specified network into the [`ShardTaskLimit`] storage.
		///    3. Emit an event indicating the shard task limit has been set.
		///    4. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::set_shard_task_limit())]
		pub fn set_shard_task_limit(
			origin: OriginFor<T>,
			network: NetworkId,
			limit: u32,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			ShardTaskLimit::<T>::insert(network, limit);
			Self::deposit_event(Event::ShardTaskLimitSet(network, limit));
			Ok(())
		}

		/// Unregisters a specified number of gateways.
		///
		/// # Flow
		///   1. Ensure the origin of the transaction is a root user.
		///   2. Clear the specified number of gateways from the `Gateway` storage.
		///   3. Clear all registered shards from the [`ShardRegistered`] storage.
		///   4. Filter and process tasks, finishing tasks with a [`Function::ReadMessages`] function with an error result indicating shard offline or gateway change.
		///   5. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(12)]
		#[pallet::weight(<T as Config>::WeightInfo::unregister_gateways(*num_gateways))]
		pub fn unregister_gateways(origin: OriginFor<T>, num_gateways: u32) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let _ = Gateway::<T>::clear(num_gateways, None);
			// safest to keep this clear_all or add additional weight hint in
			// follow up. Iterating through only the removed gateways would complicate
			// things
			let _ = ShardRegistered::<T>::clear(u32::MAX, None);
			Self::filter_tasks(|task_id| {
				let Some(task) = Tasks::<T>::get(task_id) else {
					return;
				};
				if let Function::ReadMessages { .. } = task.function {
					Self::finish_task(
						task_id,
						TaskResult {
							shard_id: 0,
							payload: Payload::Error("shard offline or gateway changed".into()),
							signature: [0u8; 64],
						},
					);
				}
			});
			Ok(())
		}

		/// Sets the batch size and offset for a specific network.
		///
		/// # Flow
		///   1. Ensure the origin of the transaction is a root user.
		///   2. Insert the new batch size for the specified network into the [`NetworkBatchSize`] storage.
		///   3. Insert the new offset for the specified network into the [`NetworkOffset`] storage.
		///   4. Emit an event indicating the batch size and offset have been set.
		///   5. Return `Ok(())` if all operations succeed.
		#[pallet::call_index(13)]
		#[pallet::weight(<T as Config>::WeightInfo::set_batch_size())]
		pub fn set_batch_size(
			origin: OriginFor<T>,
			network: NetworkId,
			batch_size: u64,
			offset: u64,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			NetworkBatchSize::<T>::insert(network, batch_size);
			NetworkOffset::<T>::insert(network, offset);
			Self::deposit_event(Event::BatchSizeSet(network, batch_size, offset));
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			Self::schedule_tasks()
		}
	}

	impl<T: Config> Pallet<T> {
		/// Retrieves the TSS (Threshold Signature Scheme) signature for a given task.
		/// Look up the `TssSignature` associated with the provided `task` ID in the storage.
		pub fn get_task_signature(task: TaskId) -> Option<TssSignature> {
			TaskSignature::<T>::get(task)
		}

		/// Retrieves the public key of the signer for a given task.
		/// Look up the `PublicKey` of the signer associated with the provided `task` ID in the storage.
		pub fn get_task_signer(task: TaskId) -> Option<PublicKey> {
			TaskSigner::<T>::get(task)
		}

		/// Retrieves the hash for a given task.
		/// Look up the hash associated with the provided `task` ID in the storage.
		pub fn get_task_hash(task: TaskId) -> Option<[u8; 32]> {
			TaskHash::<T>::get(task)
		}

		/// Retrieves a list of tasks associated with a given shard.
		/// Look up the tasks associated with the provided `shard_id` in the storage.
		pub fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution> {
			ShardTasks::<T>::iter_prefix(shard_id)
				.map(|(task_id, _)| TaskExecution::new(task_id, TaskPhaseState::<T>::get(task_id)))
				.collect()
		}

		/// Retrieves the descriptor for a given task.
		/// Look up the `TaskDescriptor` associated with the provided `task_id` in the storage.
		pub fn get_task(task_id: TaskId) -> Option<TaskDescriptor> {
			Tasks::<T>::get(task_id)
		}

		/// Retrieves the gateway address for a given network.
		/// Look up the gateway address associated with the provided `network` ID in the storage.
		pub fn get_gateway(network: NetworkId) -> Option<[u8; 20]> {
			Gateway::<T>::get(network)
		}

		/// Retrieves the current phase of a given task.
		/// Look up the current phase of the task associated with the provided `task_id` in the storage.
		pub fn get_task_phase(task_id: TaskId) -> TaskPhase {
			TaskPhaseState::<T>::get(task_id)
		}

		/// Retrieves the shard ID associated with a given task.
		/// Look up the shard ID associated with the provided `task_id` in the storage.
		pub fn get_task_shard(task_id: TaskId) -> Option<ShardId> {
			TaskShard::<T>::get(task_id)
		}

		/// Retrieves the result of a given task.
		/// Look up the `TaskResult` associated with the provided `task_id` in the storage.
		pub fn get_task_result(task_id: TaskId) -> Option<TaskResult> {
			TaskOutput::<T>::get(task_id)
		}
	}

	impl<T: Config> Pallet<T> {
		/// The account ID containing the funds for a task.
		fn task_account(task_id: TaskId) -> T::AccountId {
			<T as Config>::PalletId::get().into_sub_account_truncating(task_id)
		}

		/// Prioritized tasks
		/// function = {UnRegisterShard, RegisterShard, ReadMessages}
		fn prioritized_unassigned_tasks(network: NetworkId) -> Box<dyn TaskQ<T>> {
			Box::new(TaskQueue::<
				UASystemTasksInsertIndex<T>,
				UASystemTasksRemoveIndex<T>,
				UnassignedSystemTasks<T>,
			>::new(network))
		}

		/// Non-prioritized tasks which are assigned only after
		/// all prioritized tasks are assigned.
		fn remaining_unassigned_tasks(network: NetworkId) -> Box<dyn TaskQ<T>> {
			Box::new(
				TaskQueue::<UATasksInsertIndex<T>, UATasksRemoveIndex<T>, UnassignedTasks<T>>::new(
					network,
				),
			)
		}

		/// Records the reception of messages for a specific network at a given block height and initiates a task to read these messages in batches.
		///
		/// # Flow
		///   1. Insert network_id and block_height into RecvTasks storage.
		///   2. Create a task descriptor with `network_id`, [`Function::ReadMessages`] { `batch_size` }, and default shard size.
		///   3. Set the task's start field to block_height.
		///   4. Start the task with [`TaskFunder::Treasury`].
		fn recv_messages(
			network_id: NetworkId,
			block_height: u64,
			batch_size: NonZeroU64,
		) -> DispatchResult {
			let mut task = TaskDescriptorParams::new(
				network_id,
				Function::ReadMessages { batch_size },
				T::Elections::default_shard_size(),
			);
			task.start = block_height;
			ensure!(
				Self::start_task(task, TaskFunder::Treasury).is_ok(),
				Error::<T>::InsufficientTreasuryBalanceToRecvTasks
			);
			RecvTasks::<T>::insert(network_id, block_height);
			Ok(())
		}

		/// Registers a new shard and initiates a task for shard registration.
		///
		/// # Flow
		///  1. Create a task descriptor with `network_id`, [`Function::RegisterShard`] { `shard_id` }, and shard member count.
		///  2. Start the task with [`TaskFunder::Treasury`].
		fn register_shard(shard_id: ShardId, network_id: NetworkId) {
			if Self::start_task(
				TaskDescriptorParams::new(
					network_id,
					Function::RegisterShard { shard_id },
					T::Shards::shard_members(shard_id).len() as _,
				),
				TaskFunder::Treasury,
			)
			.is_err()
			{
				Self::deposit_event(Event::InsufficientTreasuryBalanceToRegisterShard(shard_id));
			}
		}

		/// Filters and processes tasks using a provided function.
		///
		/// # Flow
		///   1. Iterate over all entries in [`UnassignedTasks`] and [`UnassignedSystemTasks`].
		///   2. For each entry, apply the provided function to `task_id`.
		///   3. Iterate over all entries in ShardTasks.
		///   4. For each entry, apply the provided function to task_id.
		fn filter_tasks<F: Fn(TaskId)>(f: F) {
			for (_network, _, task_id) in
				UnassignedTasks::<T>::iter().chain(UnassignedSystemTasks::<T>::iter())
			{
				f(task_id);
			}
			for (_shard_id, task_id, _) in ShardTasks::<T>::iter() {
				f(task_id);
			}
		}
		/// Unregisters a shard and processes related tasks.
		///
		/// # Flow
		///   1. Check if the shard with shard_id is registered.
		///   2.  If the shard is registered:
		///     - Start a task to unregister the shard.
		///     - Fund the task using the treasury.
		///   3. If the shard is not registered:
		///     - Iterate through existing tasks.
		///     - For each task, check if it is a registration task for the same shard.
		///   4. If a matching task is found, finish the task indicating the shard is offline or the gateway has changed.

		fn unregister_shard(shard_id: ShardId, network: NetworkId) {
			if ShardRegistered::<T>::take(shard_id).is_some() {
				if Self::start_task(
					TaskDescriptorParams::new(
						network,
						Function::UnregisterShard { shard_id },
						T::Shards::shard_members(shard_id).len() as _,
					),
					TaskFunder::Treasury,
				)
				.is_err()
				{
					Self::deposit_event(Event::InsufficientTreasuryBalanceToUnRegisterShard(
						shard_id,
					));
				}
				return;
			}
			Self::filter_tasks(|task_id| {
				let Some(task) = Tasks::<T>::get(task_id) else {
					return;
				};
				if let Function::RegisterShard { shard_id: s } = task.function {
					if s == shard_id {
						Self::finish_task(
							task_id,
							TaskResult {
								shard_id: 0,
								payload: Payload::Error("shard offline or gateway changed".into()),
								signature: [0u8; 64],
							},
						);
					}
				}
			});
		}

		/// To initiate a task that sends a message to a specified destination network.
		///
		/// # Flow
		///   1. Create task parameters using `TaskDescriptorParams::new` with the following:
		///     - Destination network from `msg.dest_network`.
		///    	- Function to send the message ([`Function::SendMessage { msg }`][Function::SendMessage]).
		///   2. Start a task with the created parameters and fund it using [`TaskFunder::Treasury`].
		///   3. Ensure the task is successfully started and funded using the chronicle.
		///   4. Return the Task ID of the started task.
		fn send_message(shard_id: ShardId, msg: Msg) -> Result<TaskId, DispatchError> {
			Self::start_task(
				TaskDescriptorParams::new(
					msg.dest_network,
					Function::SendMessage { msg },
					T::Shards::shard_members(shard_id).len() as _,
				),
				TaskFunder::Treasury,
			)
		}

		/// To initialize and start a task with the given parameters and fund it through various means.
		///
		/// # Flow
		///    1. Retrieve the current task ID from [`TaskIdCounter::<T>::get()`].
		///    2. Get the initial phase of the task from [`Function::initial_phase()`].
		///    3. Calculate read, write, and send message rewards based on the base rewards and network-specific rewards.
		///    4. Calculate the required funds based on the task's phase and shard size.
		///    5. Fund the Task:
		///      - If funded by an account: Transfer the necessary funds from the account to the task account.
		///      - If funded by a shard: Transfer the required stake from each shard member to the task account.
		///      - If funded by inflation: Issue the required funds and resolve them to the task account.
		///    6. Store Task Configuration:
		///      - Insert task reward configuration into [`TaskRewardConfig::<T>`].
		///      - Insert the initial phase state into [`TaskPhaseState::<T>`].
		///    7. Increment and store the task ID counter.
		///    8. Add the task to the list of unassigned tasks for the specified network.
		///    9. Emit an event indicating the task has been created.
		///    10. Schedule the tasks for the specified network.
		fn start_task(
			schedule: TaskDescriptorParams,
			who: TaskFunder,
		) -> Result<TaskId, DispatchError> {
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
				/*TaskFunder::Account(user) => {
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
				},*/
				TaskFunder::Treasury => {
					pallet_balances::Pallet::<T>::transfer(
						&pallet_treasury::Pallet::<T>::account_id(),
						&Self::task_account(task_id),
						required_funds,
						ExistenceRequirement::KeepAlive,
					)?;
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
					function: schedule.function.clone(),
					start: schedule.start,
					shard_size: schedule.shard_size,
				},
			);
			TaskPhaseState::<T>::insert(task_id, phase);
			TaskIdCounter::<T>::put(task_id.saturating_plus_one());
			Self::add_unassigned_task(schedule.network, task_id);
			Self::deposit_event(Event::TaskCreated(task_id));
			Ok(task_id)
		}

		/// To update the phase of a task in a decentralized application, ensuring accurate tracking and execution based on its current state.
		///
		/// # Flow
		///   1. Retrieve the current block number.
		///   2. Update the task's phase state to reflect the new phase.
		///   3. Record the start of this new phase with the current block number for future reference.
		///   4. If the phase transition involves TaskPhase::Write, determine the next designated signer for the shard and store this information, preparing for subsequent actions or validations required in the workflow.
		fn start_phase(shard_id: ShardId, task_id: TaskId, phase: TaskPhase) {
			let block = frame_system::Pallet::<T>::block_number();
			TaskPhaseState::<T>::insert(task_id, phase);
			PhaseStart::<T>::insert(task_id, phase, block);
			if phase == TaskPhase::Write {
				TaskSigner::<T>::insert(task_id, T::Shards::next_signer(shard_id));
			}
		}

		/// Store the result of a task identified by task_id and update associated data if necessary.
		///
		/// # Flow
		///   1. Store result in [`TaskOutput::<T>`] for the specified task_id.
		///   2. If `task_id` has an associated `shard_id`, remove `task_id` from [`ShardTasks::<T>`].
		fn finish_task(task_id: TaskId, result: TaskResult) {
			TaskOutput::<T>::insert(task_id, result);
			if let Some(shard_id) = TaskShard::<T>::take(task_id) {
				ShardTasks::<T>::remove(shard_id, task_id);
			}
		}

		/// Cancel a task identified by `task_id` on a specified network.
		///   
		/// # Flow
		///   1. Create a `TaskResult` indicating the task cancellation due to administrative action.
		///   2. Finalize the task by storing the cancellation result using [`Self::finish_task`].
		///   3. If `task_id` is indexed as an unassigned task ([`UATaskIndex::<T>`]), remove it from the unassigned task index for the specified `task_network`.
		///   4. Remove all stored task-related data:
		///     - Clear the task's phase state ([`TaskPhaseState::<T>::remove`]).
		///     - Remove the designated signer for the task ([`TaskSigner::<T>::remove`]).
		///     - Clear any stored task signature ([`TaskSignature::<T>::remove`]).
		///     - Remove any associated task hash ([`TaskHash::<T>::remove`]).
		///   5. Emit an event ([`Event::TaskResult`]) indicating the cancellation and its result.
		fn cancel_task(task_id: TaskId, task_network: NetworkId) {
			let result = TaskResult {
				shard_id: 0,
				payload: Payload::Error("task cancelled by sudo".into()),
				signature: [0; 64],
			};
			Self::finish_task(task_id, result.clone());
			if let Some(task_index) = UATaskIndex::<T>::take(task_id) {
				Self::remove_unassigned_task(task_network, task_index, task_id);
			}
			TaskPhaseState::<T>::remove(task_id);
			TaskSigner::<T>::remove(task_id);
			TaskSignature::<T>::remove(task_id);
			TaskHash::<T>::remove(task_id);
			Self::deposit_event(Event::TaskResult(task_id, result));
		}

		/// Validate a TSS (Threshold Signature Scheme) signature for data associated with a specific shard.
		///
		/// # Flow
		///   1. Retrieve the TSS public key for `shard_id`.
		///   2. Convert the provided `signature` into a [`schnorr_evm::Signature`].
		///   3. Convert the retrieved public key into a [`schnorr_evm::VerifyingKey`].
		///   4. Verify the `signature` against the `data` using the verifying key.
		///   5. Return `Ok(())` if verification succeeds, or an appropriate error if any step fails.
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

		/// Schedule tasks for a specified network, optionally targeting a specific shard if provided.
		///   
		/// # Flow
		/// for network in networks:
		/// 	tasks_per_shard = (assigned_tasks(network) + unassigned_tasks(network)) / number_of_registered_shards(network)
		/// 	tasks_per_shard = min(tasks_per_shard, max_assignable_tasks)
		/// 	for registered_shard in network:
		/// 		number_of_tasks_to_assign = min(tasks_per_shard, shard_capacity(registered_shard))
		fn schedule_tasks() -> Weight {
			const DEFAULT_SHARD_TASK_LIMIT: u32 = 10;
			// To account for any computation involved outside of accounted reads/writes
			// Overestimating can lead to more consistent block times especially if weight was underestimated prior to adding the safety margin
			const WEIGHT_SAFETY_MARGIN: Weight = Weight::from_parts(500_000_000, 0);
			let mut weight = Weight::default();
			for (network, _) in Gateway::<T>::iter() {
				// for this network, compute unassigned tasks count for this network
				let unassigned_tasks = UnassignedTasks::<T>::iter_prefix(network)
					.count()
					.saturating_add(UnassignedSystemTasks::<T>::iter_prefix(network).count());
				// READs: Gateway, UnassignedTasks, UnassignedSystemTasks
				weight = weight.saturating_add(T::DbWeight::get().reads(3));
				// for this network, compute assigned tasks count and registered shards count
				let (mut assigned_tasks, mut registered_shards) = (0usize, Vec::new());
				for (shard, _) in NetworkShards::<T>::iter_prefix(network) {
					if ShardRegistered::<T>::get(shard).is_some() {
						registered_shards.push(shard);
					}
					assigned_tasks = assigned_tasks.saturating_add(
						ShardTasks::<T>::iter_prefix(shard)
							.filter(|(task, _)| TaskOutput::<T>::get(task).is_none())
							.count(),
					);
					// READs: NetworkShards, ShardRegistered, UnassignedSystemTasks
					weight = weight.saturating_add(T::DbWeight::get().reads(3));
				}
				// for this network, assign 0 tasks if 0 registered shards
				if registered_shards.is_empty() {
					continue;
				}
				// for this network, compute a number of tasks per shard to balance task allocation
				let mut tasks_per_shard = (assigned_tasks.saturating_add(unassigned_tasks))
					.saturating_div(registered_shards.len());
				let max_assignable_tasks =
					ShardTaskLimit::<T>::get(network).unwrap_or(DEFAULT_SHARD_TASK_LIMIT) as usize;
				tasks_per_shard = sp_std::cmp::min(tasks_per_shard, max_assignable_tasks);
				// READs: ShardTaskLimit
				weight = weight.saturating_add(T::DbWeight::get().reads(1));
				// for this network, assign unassigned tasks to registered shards evenly
				for shard in registered_shards {
					let shard_capacity = max_assignable_tasks
						.saturating_sub(ShardTasks::<T>::iter_prefix(shard).count());
					let tasks_for_shard = sp_std::cmp::min(tasks_per_shard, shard_capacity);
					weight = weight.saturating_add(T::DbWeight::get().reads(1));
					weight = weight
						.saturating_add(Self::schedule_tasks_shard(network, shard, tasks_for_shard))
						// READS: ShardTasks
						.saturating_add(T::DbWeight::get().reads(1));
				}
			}
			weight.saturating_add(WEIGHT_SAFETY_MARGIN)
		}

		/// To schedule tasks for a specified network and optionally for a specific shard, optimizing
		/// task allocation based on current workload and system constraints.
		///   
		/// # Flow
		///   1. Count the number of incomplete tasks (`tasks`) for the specified `shard_id`.
		///   2. Determine the size of the shard (`shard_size`) based on the number of shard members.
		///   3. Check if the `shard_id` is registered.
		///   4. Retrieve the maximum allowed tasks (`shard_task_limit`) for the network, defaulting to 10 if unspecified.
		///   5. Calculate the remaining capacity (`capacity`) for new tasks.
		///   6. If `capacity` is zero, stop further task assignments.
		///   7. Get system tasks and, if space permits, non-system tasks.
		///   8. Assign each task to the shard using `Self::assign_task(network, shard_id, index, task)`.
		fn schedule_tasks_shard(network: NetworkId, shard_id: ShardId, capacity: usize) -> Weight {
			let mut reads = 0;
			let system_tasks = Self::prioritized_unassigned_tasks(network).get_n(capacity);
			let tasks = if let Some(non_system_capacity) = capacity.checked_sub(system_tasks.len())
			{
				let non_system_tasks =
					Self::remaining_unassigned_tasks(network).get_n(non_system_capacity);
				// reads: remaining_unassigned_tasks
				reads = reads.saturating_plus_one();
				system_tasks.into_iter().chain(non_system_tasks).collect::<Vec<_>>()
			} else {
				system_tasks
			};
			// reads: T::Shards::shard_members, ShardRegistered, prioritized_unassigned_tasks
			reads = reads.saturating_add(3);
			let mut weight = T::DbWeight::get().reads(reads);
			for (index, task) in tasks {
				weight = weight.saturating_add(Self::assign_task(network, shard_id, index, task));
			}
			weight
		}

		/// Assign a task to a specific shard within a network, managing task allocation and phase initialization.
		///   
		/// # Flow
		///   1. If the task was previously assigned to another shard, remove it from there.
		///   2. Remove the task from the list of unassigned tasks.
		///   3. Assign the task to the new shard.
		///   4. Update the task's shard assignment.
		///   5. Start the task's phase.
		fn assign_task(
			network: NetworkId,
			shard_id: ShardId,
			task_index: TaskIndex,
			task_id: TaskId,
		) -> Weight {
			let (mut reads, mut writes) = (0, 0);
			if let Some(old_shard_id) = TaskShard::<T>::get(task_id) {
				ShardTasks::<T>::remove(old_shard_id, task_id);
				// writes: ShardTasks
				writes = writes.saturating_plus_one();
			}
			Self::remove_unassigned_task(network, task_index, task_id);
			ShardTasks::<T>::insert(shard_id, task_id, ());
			TaskShard::<T>::insert(task_id, shard_id);
			Self::start_phase(shard_id, task_id, TaskPhaseState::<T>::get(task_id));
			// writes: remove_unassigned_task, ShardTasks, TaskShard, start_phase
			writes = writes.saturating_add(4);
			// reads: TaskShard, TaskPhaseState
			reads = reads.saturating_add(2);
			T::DbWeight::get()
				.reads(reads)
				.saturating_add(T::DbWeight::get().writes(writes))
		}

		/// Applies the depreciation rate to calculate the remaining reward.
		///
		/// # Flow
		///   1. Calculate the remaining reward by applying the depreciation rate to the initial reward amount.
		///   2. Return the remaining reward.
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

		/// Records the write reward for a task signer.
		///
		/// # Flow
		///   1. Record the reward amount for the task signer in the appropriate storage.
		///   2. Return `Ok(())` if the operation succeeds.
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

		/// Distributes rewards to shard members and signers upon task completion.
		///
		/// # Flow
		///   1. Calculate the total reward for the task.
		///   2. Distribute the rewards to the shard members and signers based on predefined rules.
		///   3. Return `Ok(())` if the operation succeeds.
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

		/// Manage the assignment of tasks to networks based on their function type, prioritizing system tasks differently from others.
		///   
		/// # Flow
		///   1. Retrieve the task `task` associated with `task_id` from storage.
		///   2. Check the function type of the task:
		///     - If it is a system task [`Function::UnregisterShard`], [`Function::RegisterShard`], [`Function::ReadMessages`], add `task_id` to the prioritized unassigned tasks list for the `network`.
		///     - Otherwise, add `task_id` to the remaining unassigned tasks list for the `network`.
		pub fn add_unassigned_task(network: NetworkId, task_id: TaskId) {
			let Some(task) = Tasks::<T>::get(task_id) else { return };
			match task.function {
				// system tasks
				Function::UnregisterShard { .. }
				| Function::RegisterShard { .. }
				| Function::ReadMessages { .. } => {
					Self::prioritized_unassigned_tasks(network).push(task_id);
				},
				_ => {
					Self::remaining_unassigned_tasks(network).push(task_id);
				},
			}
		}

		/// Removes an unassigned task from the appropriate list based on its function type.
		///
		/// # Flow
		///   1. Retrieve the task associated with task_id
		///   2.  Determine the function type of the task
		///   3.  Remove system tasks from the prioritized list
		///   4.  Remove other tasks from the remaining list
		pub fn remove_unassigned_task(network: NetworkId, task_index: TaskIndex, task_id: TaskId) {
			let Some(task) = Tasks::<T>::get(task_id) else { return };
			match task.function {
				// system tasks
				Function::UnregisterShard { .. }
				| Function::RegisterShard { .. }
				| Function::ReadMessages { .. } => {
					Self::prioritized_unassigned_tasks(network).remove(task_index);
				},
				_ => {
					Self::remaining_unassigned_tasks(network).remove(task_index);
				},
			}
		}
	}

	impl<T: Config> TasksInterface for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: NetworkId) {
			NetworkShards::<T>::insert(network, shard_id, ());
			if Gateway::<T>::get(network).is_some() {
				Self::register_shard(shard_id, network);
			}
		}

		fn shard_offline(shard_id: ShardId, network: NetworkId) {
			NetworkShards::<T>::remove(network, shard_id);
			// unassign tasks
			ShardTasks::<T>::drain_prefix(shard_id).for_each(|(task_id, _)| {
				TaskShard::<T>::remove(task_id);
				Self::add_unassigned_task(network, task_id);
			});
			Self::unregister_shard(shard_id, network);
		}
	}
}
