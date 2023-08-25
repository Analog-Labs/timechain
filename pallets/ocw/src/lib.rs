#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
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
	use sp_runtime::offchain::storage::StorageValueRef;
	use sp_runtime::traits::{Block, Header, IdentifyAccount};
	use sp_std::vec;
	use time_primitives::{
		msg_key, AccountId, CycleStatus, Network, OcwPayload, OcwShardInterface,
		OcwSubmitTaskResult, PublicKey, ShardCreated, ShardId, TaskCycle, TaskError, TaskId,
		TssPublicKey, OCW_READ_ID, OCW_STATUS, OCW_WRITE_ID,
	};

	pub trait WeightInfo {
		fn submit_tss_public_key() -> Weight;
		fn submit_task_result() -> Weight;
		fn set_shard_offline() -> Weight;
		fn submit_task_error() -> Weight;
	}

	impl WeightInfo for () {
		fn submit_tss_public_key() -> Weight {
			Weight::default()
		}
		fn submit_task_result() -> Weight {
			Weight::default()
		}
		fn set_shard_offline() -> Weight {
			Weight::default()
		}
		fn submit_task_error() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: <<T::Block as Block>::Header as Header>::Number) {
			if Self::is_free() {
				log::info!("running offchain worker for: {:?}", block_number);
				while let Some(msg) = Self::read_message() {
					log::info!("received ocw message {:?}", msg);
					Self::submit_tx(msg);
				}
				Self::set_free(true);
			}
			log::info!("finished offchain worker for: {:?}", block_number);
		}
	}

	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<Call<Self>, Public = PublicKey>
		+ frame_system::Config<AccountId = AccountId>
	{
		type WeightInfo: WeightInfo;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type Shards: OcwShardInterface;
		type Tasks: OcwSubmitTaskResult;
	}

	#[pallet::storage]
	pub type ShardCollector<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, PublicKey, OptionQuery>;

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		NotSignedByCollector,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submits TSS group key to runtime
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::submit_tss_public_key())]
		pub fn submit_tss_public_key(
			origin: OriginFor<T>,
			shard_id: ShardId,
			public_key: TssPublicKey,
		) -> DispatchResult {
			Self::ensure_signed_by_collector(origin, shard_id)?;
			T::Shards::submit_tss_public_key(shard_id, public_key)
		}

		/// Submits task result to runtime
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_task_result())]
		pub fn submit_task_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: TaskCycle,
			status: CycleStatus,
		) -> DispatchResult {
			Self::ensure_signed_by_collector(origin, status.shard_id)?;
			T::Tasks::submit_task_result(task_id, cycle, status)
		}

		/// Turns shard offline
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_shard_offline())]
		pub fn set_shard_offline(
			origin: OriginFor<T>,
			shard_id: ShardId,
			network: Network,
		) -> DispatchResult {
			Self::ensure_signed_by_collector(origin, shard_id)?;
			T::Shards::set_shard_offline(shard_id, network)
		}

		/// Submit Task Error
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::submit_task_error())]
		pub fn submit_task_error(
			origin: OriginFor<T>,
			task_id: TaskId,
			error: TaskError,
		) -> DispatchResult {
			Self::ensure_signed_by_collector(origin, error.shard_id)?;
			T::Tasks::submit_task_error(task_id, error)
		}
	}

	impl<T: Config> Pallet<T> {
		fn ensure_signed_by_collector(origin: OriginFor<T>, shard_id: ShardId) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let Some(collector) = ShardCollector::<T>::get(shard_id) else {
				return Err(Error::<T>::NotSignedByCollector.into());
			};
			ensure!(account_id == collector.into_account(), Error::<T>::NotSignedByCollector);
			Ok(())
		}

		pub(crate) fn is_free() -> bool {
			let storage = StorageValueRef::persistent(OCW_STATUS);
			let status = storage.get::<bool>().unwrap().unwrap_or(true);
			if status {
				storage.set(&false);
			}
			return status;
		}

		pub(crate) fn set_free(status: bool) {
			let storage = StorageValueRef::persistent(OCW_STATUS);
			storage.set(&status);
		}

		pub(crate) fn read_message() -> Option<OcwPayload> {
			let read_id_storage = StorageValueRef::persistent(OCW_READ_ID);
			let write_id_storage = StorageValueRef::persistent(OCW_WRITE_ID);
			let read_id = read_id_storage.get::<u64>().unwrap().unwrap_or_default();
			let write_id = write_id_storage.get::<u64>().unwrap().unwrap_or_default();
			if read_id >= write_id {
				return None;
			}
			let msg_key = msg_key(read_id);
			let mut msg_storage = StorageValueRef::persistent(&msg_key);
			let msg = msg_storage.get::<OcwPayload>().unwrap().unwrap();
			read_id_storage
				.mutate::<u64, _, _>(|res| Ok::<_, ()>(res.unwrap().unwrap_or_default() + 1))
				.unwrap();
			msg_storage.clear();
			Some(msg)
		}

		pub(crate) fn submit_tx(payload: OcwPayload) {
			let Some(collector) = ShardCollector::<T>::get(payload.shard_id()) else {
				return;
			};
			let signer = Signer::<T, T::AuthorityId>::any_account().with_filter(vec![collector]);
			let call_res = match payload {
				OcwPayload::SubmitTssPublicKey { shard_id, public_key } => signer
					.send_signed_transaction(|_| Call::submit_tss_public_key {
						shard_id,
						public_key,
					}),
				OcwPayload::SubmitTaskResult { task_id, cycle, status } => signer
					.send_signed_transaction(|_| Call::submit_task_result {
						task_id,
						cycle,
						status: status.clone(),
					}),

				OcwPayload::SetShardOffline { shard_id, network } => signer
					.send_signed_transaction(|_| Call::set_shard_offline { shard_id, network }),
				OcwPayload::SubmitTaskError { task_id, error } => {
					signer.send_signed_transaction(|_| Call::submit_task_error {
						task_id,
						error: error.clone(),
					})
				},
			};
			let Some((_, res)) = call_res else {
				log::info!("send signed transaction returned none");
				return;
			};
			if let Err(e) = res {
				log::error!("send signed transaction returned an error: {:?}", e);
			}
		}
	}

	impl<T: Config> ShardCreated for Pallet<T> {
		fn shard_created(shard_id: ShardId, collector: PublicKey) {
			ShardCollector::<T>::insert(shard_id, collector);
		}
	}
}
