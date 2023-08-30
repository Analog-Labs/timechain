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
	use scale_info::prelude::string::String;
	use sp_runtime::traits::IdentifyAccount;
	use sp_std::vec;
	use time_primitives::{
		AccountId, CycleStatus, OcwPayload, OcwShardInterface, OcwTaskInterface, PublicKey,
		ShardId, ShardsInterface, TaskCycle, TaskError, TaskId, TasksInterface, TssPublicKey,
	};

	pub trait WeightInfo {
		fn submit_tss_public_key() -> Weight;
		fn submit_task_hash() -> Weight;
		fn submit_task_result() -> Weight;
		fn submit_task_error() -> Weight;
	}

	impl WeightInfo for () {
		fn submit_tss_public_key() -> Weight {
			Weight::default()
		}
		fn submit_task_hash() -> Weight {
			Weight::default()
		}
		fn submit_task_result() -> Weight {
			Weight::default()
		}
		fn submit_task_error() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<Call<Self>, Public = PublicKey>
		+ frame_system::Config<AccountId = AccountId>
	{
		type WeightInfo: WeightInfo;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type OcwShards: OcwShardInterface;
		type OcwTasks: OcwTaskInterface;
		type Shards: ShardsInterface;
		type Tasks: TasksInterface;
	}

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
			ensure_none(origin)?;
			T::OcwShards::submit_tss_public_key(shard_id, public_key)
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
			ensure_none(origin)?;
			T::OcwTasks::submit_task_result(task_id, cycle, status)
		}

		/// Submit Task Error
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::submit_task_error())]
		pub fn submit_task_error(
			origin: OriginFor<T>,
			task_id: TaskId,
			error: TaskError,
		) -> DispatchResult {
			ensure_none(origin)?;
			T::OcwTasks::submit_task_error(task_id, error)
		}

		/// Submit Task Hash
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::submit_task_hash())]
		pub fn submit_task_hash(
			origin: OriginFor<T>,
			shard_id: ShardId,
			task_id: TaskId,
			hash: String,
		) -> DispatchResult {
			Self::ensure_signed_by_collector(origin, shard_id)?;
			T::OcwTasks::submit_task_hash(shard_id, task_id, hash)
		}
	}

	impl<T: Config> Pallet<T> {
		fn ensure_signed_by_collector(origin: OriginFor<T>, shard_id: ShardId) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let Some(collector) = T::Shards::collector_pubkey(shard_id) else {
				return Err(Error::<T>::NotSignedByCollector.into());
			};
			ensure!(account_id == collector.into_account(), Error::<T>::NotSignedByCollector);
			Ok(())
		}

		pub fn submit_signed_tx(payload: OcwPayload) {
			let Some(collector) = T::Shards::collector_pubkey(payload.shard_id()) else {
				return;
			};
			let signer = Signer::<T, T::AuthorityId>::any_account().with_filter(vec![collector]);

			let call_res = match payload {
				OcwPayload::SubmitTaskHash { shard_id, task_id, hash } => signer
					.send_signed_transaction(|_| Call::submit_task_hash {
						shard_id,
						task_id,
						hash: hash.clone(),
					}),
				_ => {
					log::error!("needed unsigned payload got signed {:?}", payload);
					None
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

		pub fn submit_unsigned_tx(payload: OcwPayload) {
			use frame_system::offchain::SubmitTransaction;

			let call = match payload.clone() {
				OcwPayload::SubmitTssPublicKey { shard_id, public_key } => {
					Call::submit_tss_public_key { shard_id, public_key }
				},
				OcwPayload::SubmitTaskResult { task_id, cycle, status } => {
					Call::submit_task_result { task_id, cycle, status }
				},
				OcwPayload::SubmitTaskError { task_id, error } => {
					Call::submit_task_error { task_id, error }
				},
				_ => {
					log::error!("payload needed go be signed {:?}", payload);
					return;
				},
			};
			let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
			match res {
				Ok(_) => log::info!("Submitted unsigned tx {:?}", payload),
				Err(e) => {
					log::error!("Error submitting unsigned tx {:?}: {:?}", payload, e)
				},
			}
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		fn validate_unsigned(source: TransactionSource, _call: &Self::Call) -> TransactionValidity {
			if !matches!(source, TransactionSource::Local | TransactionSource::InBlock) {
				return InvalidTransaction::Call.into();
			}
			ValidTransaction::with_tag_prefix("ocw-pallet")
				.priority(TransactionPriority::max_value())
				.longevity(3)
				.propagate(true)
				.build()
		}

		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}
	}
}
