#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
	};
	use frame_system::pallet_prelude::*;
	use time_primitives::{
		OcwPayload, ScheduleCycle, ScheduleInterface, ScheduleStatus, ShardId, ShardInterface,
		TaskId, TssPublicKey,
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
			while let Some(msg) = Self::read_message() {
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
		type Shards: ShardInterface;
		type Tasks: ScheduleInterface;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
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
				Some(account_id) == T::Shards::collector(shard_id),
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
				Some(account_id) == T::Shards::collector(status.shard_id),
				Error::<T>::NotSignedByCollector
			);
			T::Tasks::submit_task_result(task_id, cycle, status)
		}
	}

	impl<T: Config> Pallet<T> {
		fn read_message() -> Option<OcwPayload> {
			None
		}

		fn submit_tx(payload: OcwPayload) {
			let Some(_collector) = T::Shards::collector(payload.shard_id()) else {
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
				OcwPayload::SubmitResult { task_id, cycle, status } => signer
					.send_signed_transaction(|_| Call::submit_task_result {
						task_id,
						cycle,
						status: status.clone(),
					}),
			};
			let Some((_, res)) = call_res else {
				log::error!("send signed transaction returned none");
				return;
			};
			if let Err(e) = res {
				log::error!("send signed transaction returned an error: {:?}", e);
			}
		}
	}
}
