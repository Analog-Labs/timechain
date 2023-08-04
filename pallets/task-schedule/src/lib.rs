#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use pallet::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::offchain::AppCrypto;
	use frame_system::offchain::CreateSignedTransaction;
	use frame_system::offchain::SendSignedTransaction;
	use frame_system::offchain::Signer;
	use frame_system::pallet_prelude::*;
	use sp_runtime::offchain::storage::MutateStorageError;
	use sp_runtime::offchain::storage::StorageRetrievalError;
	use sp_runtime::offchain::storage::StorageValueRef;
	use sp_runtime::traits::AppVerify;
	use sp_std::collections::vec_deque::VecDeque;
	use sp_std::vec::Vec;
	use time_primitives::OCWPayload;
	use time_primitives::OCWSkdData;
	use time_primitives::ShardInterface;
	use time_primitives::OCW_SKD_KEY;
	use time_primitives::SkdMsg;
use time_primitives::{
		Network, ScheduleCycle, ScheduleInput, ScheduleInterface, ScheduleStatus, ShardId, TaskId,
		TaskSchedule,
	};
	use time_primitives::crypto::Signature;

	pub trait WeightInfo {
		fn create_task() -> Weight;
		fn submit_result() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(_block_number: T::BlockNumber) {
			Self::ocw_get_skd_data();
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
		type ShardHelper: ShardInterface;
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
	pub type Tasks<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, TaskSchedule, OptionQuery>;

	#[pallet::storage]
	pub type TaskCycle<T: Config> =
		StorageMap<_, Blake2_128Concat, TaskId, ScheduleCycle, ValueQuery>;

	#[pallet::storage]
	pub type TaskResults<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TaskId,
		Blake2_128Concat,
		ScheduleCycle,
		ScheduleStatus,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		TaskCreated(TaskId),
		/// Updated Schedule
		TaskResult(TaskId, ScheduleCycle, ScheduleStatus),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Invalid cycle
		InvalidCycle,
		// Failed to send offchain transaction
		OffchainTxFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_task())]
		pub fn create_task(origin: OriginFor<T>, schedule: ScheduleInput) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let task_id = TaskIdCounter::<T>::get();
			Tasks::<T>::insert(
				task_id,
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
			TaskIdCounter::<T>::put(task_id + 1);
			UnassignedTasks::<T>::insert(schedule.network, task_id, ());
			Self::deposit_event(Event::TaskCreated(task_id));
			Self::schedule_tasks(schedule.network);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_result())]
		pub fn submit_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: ScheduleCycle,
			status: ScheduleStatus,
			proof: Signature
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(TaskCycle::<T>::get(task_id) == cycle, Error::<T>::InvalidCycle);
			Self::is_collector_and_signed(proof, task_id, cycle, status.clone());
			TaskCycle::<T>::insert(task_id, cycle + 1);
			TaskResults::<T>::insert(task_id, cycle, status.clone());
			if Self::is_complete(task_id) {
				if let Some(shard_id) = TaskShard::<T>::get(task_id) {
					ShardTasks::<T>::remove(shard_id, task_id);
				}
				TaskShard::<T>::remove(task_id);
			}
			Self::deposit_event(Event::TaskResult(task_id, cycle, status));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_shard_tasks(shard_id: ShardId) -> Vec<(TaskId, ScheduleCycle)> {
			ShardTasks::<T>::iter_prefix(shard_id)
				.map(|(task_id, _)| (task_id, TaskCycle::<T>::get(task_id)))
				.collect()
		}

		pub fn get_task(task_id: TaskId) -> Option<TaskSchedule> {
			Tasks::<T>::get(task_id)
		}
	}

	impl<T: Config> Pallet<T> {
		fn is_complete(task_id: TaskId) -> bool {
			if let Some(task) = Tasks::<T>::get(task_id) {
				TaskResults::<T>::contains_key(task_id, task.cycle)
			} else {
				true
			}
		}

		fn shard_task_count(shard_id: ShardId) -> usize {
			ShardTasks::<T>::iter_prefix(shard_id).count()
		}

		fn schedule_tasks(network: Network) {
			for (task_id, _) in UnassignedTasks::<T>::iter_prefix(network) {
				let shard = NetworkShards::<T>::iter_prefix(network)
					.map(|(shard_id, _)| (shard_id, Self::shard_task_count(shard_id)))
					.reduce(|(shard_id, task_count), (shard_id2, task_count2)| {
						if task_count < task_count2 {
							(shard_id, task_count)
						} else {
							(shard_id2, task_count2)
						}
					});
				let Some((shard_id, _)) = shard else {
					break;
				};
				ShardTasks::<T>::insert(shard_id, task_id, ());
				TaskShard::<T>::insert(task_id, shard_id);
				UnassignedTasks::<T>::remove(network, task_id);
			}
		}

		fn is_collector_and_signed(proof: Signature, task_id: TaskId, cycle: ScheduleCycle, status: ScheduleStatus) -> bool {
			let msg = SkdMsg::new(task_id, cycle, status.clone()).encode();
			let msg_bytes: &[u8] = &msg;
			let Some(collector) = T::ShardHelper::get_collector(*status.shard_id()) else{
				return false;
			};
			let collector = sp_application_crypto::sr25519::Public::from_raw(collector.into());
			proof.verify(msg_bytes, &collector.into())
		}

		fn ocw_get_skd_data() {
			let storage_ref = StorageValueRef::persistent(OCW_SKD_KEY);

			const EMPTY_DATA: () = ();

			let mut tx_requests: VecDeque<OCWSkdData> = Default::default();

			let outer_res = storage_ref.mutate(
				|res: Result<Option<VecDeque<OCWPayload>>, StorageRetrievalError>| {
					match res {
						Ok(Some(mut data)) => {
							// iteration batch of 5
							for _ in 0..5 {
								let Some(data_vec) = data.pop_front() else {
									break;
								};

								let OCWPayload::OCWSkd(skd_req) = data_vec else {
									continue;
								};

								tx_requests.push_back(skd_req.clone());
							}
							Ok(data)
						},
						Ok(None) => Err(EMPTY_DATA),
						Err(_) => Err(EMPTY_DATA),
					}
				},
			);

			log::info!("updated value after skd submission {:?}", outer_res);

			match outer_res {
				Err(MutateStorageError::ValueFunctionFailed(EMPTY_DATA)) => {
					log::info!("Task schedule OCW is empty");
				},
				Err(MutateStorageError::ConcurrentModification(_)) => {
					log::error!("💔 Error updating local storage in SKD OCW",);
				},
				Ok(_) => {},
			}

			for tx in tx_requests {
				//check for collector
				if let Err(err) = Self::ocw_update_schedule_by_key(tx) {
					log::error!("Error occured while submitting extrinsic {:?}", err);
				}
			}
		}

		fn ocw_update_schedule_by_key(data: OCWSkdData) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();

			if let Some((acc, res)) =
				signer.send_signed_transaction(|_account| Call::submit_result {
					task_id: data.task_id,
					cycle: data.cycle,
					status: data.status.clone(),
					proof: data.proof.clone(),
				}) {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
					return Err(Error::OffchainTxFailed);
				} else {
					log::info!("success: offchain_signed_tx: tx sent: {:?}", acc.id);
				}
			}
			return Ok(());
		}
	}

	impl<T: Config> ScheduleInterface for Pallet<T> {
		fn shard_online(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::insert(network, shard_id, ());
			Self::schedule_tasks(network);
		}

		fn shard_offline(shard_id: ShardId, network: Network) {
			NetworkShards::<T>::remove(network, shard_id);
			ShardTasks::<T>::iter_prefix(shard_id).for_each(|(task_id, _)| {
				ShardTasks::<T>::remove(shard_id, task_id);
				TaskShard::<T>::remove(task_id);
				if !Self::is_complete(task_id) {
					UnassignedTasks::<T>::insert(network, task_id, ());
				}
			});
			Self::schedule_tasks(network);
		}
	}
}
