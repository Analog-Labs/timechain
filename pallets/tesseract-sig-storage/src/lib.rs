#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::{ValueQuery, *};
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
	use time_primitives::OCWTSSGroupKeyData;
	use time_primitives::ShardInterface;
	use time_primitives::OCW_TSS_KEY;
	use time_primitives::{crypto::Signature, Network, ScheduleInterface, ShardId, TimeId, TssPublicKey};

	pub trait WeightInfo {
		fn register_shard() -> Weight;
		fn submit_tss_group_key(_s: u32) -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(_block_number: T::BlockNumber) {
			Self::ocw_get_tss_data();
		}
	}

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type TaskScheduler: ScheduleInterface;
	}

	#[pallet::storage]
	#[pallet::getter(fn shard_id)]
	/// Counter for creating unique shard_ids during on-chain creation
	pub type ShardIdCounter<T: Config> = StorageValue<_, ShardId, ValueQuery>;

	/// Network for which shards can be assigned tasks
	#[pallet::storage]
	#[pallet::getter(fn shard_network)]
	pub type ShardNetwork<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, Network, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_public_key)]
	pub type ShardPublicKey<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, TssPublicKey, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_collector)]
	pub type ShardCollector<T: Config> =
		StorageMap<_, Blake2_128Concat, ShardId, TimeId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn shard_members)]
	pub type ShardMembers<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ShardId, Blake2_128Concat, TimeId, (), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ShardCreated(ShardId, Network),
		/// New group key submitted to runtime
		ShardPublicKey(ShardId, [u8; 33]),
	}

	#[pallet::error]
	pub enum Error<T> {
		UnknownShard,
		PublicKeyAlreadyRegistered,
		InvalidNumberOfShardMembers,
		/// Invalid validation signature
		InvalidValidationSignature,
		/// offchain tx failed
		OffchainTxFailed,
		// no account for ocw found
		NoLocalAcctForSignedTx,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Root can register new shard via providing
		/// set of IDs matching one of supported size of shard
		/// # Param
		/// * members - supported sized set of shard members Id
		/// * collector - index of collector if not index 0
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::register_shard())]
		pub fn register_shard(
			origin: OriginFor<T>,
			members: Vec<TimeId>,
			network: Network,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(members.len() == 3, Error::<T>::InvalidNumberOfShardMembers);
			let shard_id = <ShardIdCounter<T>>::get();
			<ShardIdCounter<T>>::put(shard_id + 1);
			<ShardNetwork<T>>::insert(shard_id, network);
			<ShardCollector<T>>::insert(shard_id, members[0].clone());
			for member in members {
				<ShardMembers<T>>::insert(shard_id, member, ());
			}
			Self::deposit_event(Event::ShardCreated(shard_id, network));
			Ok(())
		}

		/// Submits TSS group key to runtime
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::submit_tss_group_key(1))]
		pub fn submit_tss_group_key(
			origin: OriginFor<T>,
			shard_id: ShardId,
			public_key: [u8; 33],
			proof: Signature,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let collector = ShardCollector::<T>::get(shard_id).ok_or(Error::<T>::UnknownShard)?;
			let collector = sp_application_crypto::sr25519::Public::from_raw(collector.into());
			ensure!(
				proof.verify(public_key.as_ref(), &collector.into()),
				Error::<T>::InvalidValidationSignature
			);
			let network = ShardNetwork::<T>::get(shard_id).ok_or(Error::<T>::UnknownShard)?;
			ensure!(
				ShardPublicKey::<T>::get(shard_id).is_none(),
				Error::<T>::PublicKeyAlreadyRegistered
			);
			<ShardPublicKey<T>>::insert(shard_id, public_key);
			Self::deposit_event(Event::ShardPublicKey(shard_id, public_key));
			T::TaskScheduler::shard_online(shard_id, network);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_shards(time_id: TimeId) -> Vec<ShardId> {
			ShardMembers::<T>::iter()
				.filter_map(
					|(shard_id, member, _)| {
						if member == time_id {
							Some(shard_id)
						} else {
							None
						}
					},
				)
				.collect()
		}

		pub fn get_shard_members(shard_id: ShardId) -> Vec<TimeId> {
			ShardMembers::<T>::iter_prefix(shard_id).map(|(time_id, _)| time_id).collect()
		}

		fn is_collector_and_signed(shard_id: ShardId, proof: Signature, msg: &[u8]) -> bool {
			let Some(collector) = ShardCollector::<T>::get(shard_id) else{
				return false;
			};
			let collector = sp_application_crypto::sr25519::Public::from_raw(collector.into());
			proof.verify(msg, &collector.into())
		}

		fn ocw_get_tss_data() {
			let storage_ref = StorageValueRef::persistent(OCW_TSS_KEY);

			const EMPTY_DATA: () = ();

			let mut tx_requests: VecDeque<OCWTSSGroupKeyData> = Default::default();

			let outer_res = storage_ref.mutate(
				|res: Result<Option<VecDeque<OCWPayload>>, StorageRetrievalError>| {
					match res {
						Ok(Some(mut data)) => {
							// iteration batch of 5
							for _ in 0..2 {
								let Some(tss_req_data) = data.pop_front() else{
									break;
								};

								let OCWPayload::OCWTSSGroupKey(tss_req) = tss_req_data else{
									continue;
								};

								tx_requests.push_back(tss_req);
							}
							Ok(data)
						},
						Ok(None) => Err(EMPTY_DATA),
						Err(_) => Err(EMPTY_DATA),
					}
				},
			);

			match outer_res {
				Err(MutateStorageError::ValueFunctionFailed(EMPTY_DATA)) => {
					log::info!("TSS OCW tss is empty");
				},
				Err(MutateStorageError::ConcurrentModification(_)) => {
					log::error!("💔 Error updating local storage in TSS OCW Signature",);
				},
				Ok(_) => {},
			}

			for tx in tx_requests {
				if !Self::is_collector_and_signed(tx.shard_id, tx.proof.clone(), &tx.group_key) {
					log::warn!("Node not collector to send data");
					continue;
				};

				if let Err(err) = Self::ocw_submit_tss_group_key(tx) {
					log::error!("Error occured while submitting extrinsic {:?}", err);
				};

				log::info!("Submitting OCW TSS key");
			}
		}

		fn ocw_submit_tss_group_key(data: OCWTSSGroupKeyData) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();

			if let Some((acc, res)) =
				signer.send_signed_transaction(|_account| Call::submit_tss_group_key {
					shard_id: data.shard_id,
					public_key: data.group_key,
					proof: data.proof.clone(),
				}) {
				if res.is_err() {
					log::error!("failure: offchain_tss_tx: tx sent: {:?}", acc.id);
					return Err(Error::OffchainTxFailed);
				} else {
					return Ok(());
				}
			}

			log::error!("No local account available");
			Err(Error::NoLocalAcctForSignedTx)
		}
	}

	impl<T: Config> ShardInterface for Pallet<T> {
		fn get_collector(shard_id: ShardId) -> Option<TimeId> {
			ShardCollector::<T>::get(shard_id)
		}
	}
}
