#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::offchain::storage::StorageValueRef;
	use time_primitives::{
		msg_key, OcwPayload, OcwSubmitTaskResult, OcwSubmitTssPublicKey, ScheduleCycle,
		ScheduleStatus, ShardCreated, ShardId, TaskId, TimeId, TssPublicKey, OCW_READ_ID,
		OCW_WRITE_ID,
	};

	pub trait WeightInfo {
		fn submit_tss_public_key() -> Weight;
		fn submit_task_result() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(_block_number: T::BlockNumber) {
			log::info!("running offchain worker");
			while let Some(msg) = Self::read_message() {
				log::info!("received ocw message {:?}", msg);
				Self::submit_tx(msg);
			}
		}
	}

	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<Call<Self>>
		+ frame_system::Config<AccountId = sp_runtime::AccountId32>
	{
		type WeightInfo: WeightInfo;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type Shards: OcwSubmitTssPublicKey;
		type Tasks: OcwSubmitTaskResult;
	}

	#[pallet::storage]
	pub type ShardCollector<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, TimeId, OptionQuery>;

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
			let account_id = ensure_signed(origin)?;
			ensure!(
				Some(account_id) == ShardCollector::<T>::get(shard_id),
				Error::<T>::NotSignedByCollector
			);
			T::Shards::submit_tss_public_key(shard_id, public_key)
		}

		/// Submits task result to runtime
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_task_result())]
		pub fn submit_task_result(
			origin: OriginFor<T>,
			task_id: TaskId,
			cycle: ScheduleCycle,
			status: ScheduleStatus,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			ensure!(
				Some(account_id) == ShardCollector::<T>::get(status.shard_id),
				Error::<T>::NotSignedByCollector
			);
			T::Tasks::submit_task_result(task_id, cycle, status)
		}
	}

	impl<T: Config> Pallet<T> {
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
			let Some(_collector) = ShardCollector::<T>::get(payload.shard_id()) else {
				return;
			};
			let signer = Signer::<T, T::AuthorityId>::any_account();
			//.with_filter(vec![collector]);
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
		fn shard_created(shard_id: ShardId, members: sp_std::vec::Vec<TimeId>) {
			ShardCollector::<T>::insert(shard_id, members[0].clone());
		}
	}
}
