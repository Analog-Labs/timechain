#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::dispatch::DispatchResult;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::IsSubType;
	use frame_support::traits::UnixTime;
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_core::Hasher;
	use sp_core::H256;
	use sp_core::H512;
	use sp_runtime::{
		traits::{BlakeTwo256, DispatchInfoOf, SignedExtension},
		transaction_validity::{InvalidTransaction, TransactionValidity, TransactionValidityError},
	};
	use sp_std::vec::Vec;

	use sp_std::prelude::*;

	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};

	// Error codes, used during validation before the transaction enters tx pool.

	// range: 101 - 120
	pub(crate) const ERR_INVALID_SOURCE_CHAIN_ID: u8 = 101;
	pub(crate) const ERR_INVALID_DEST_CHAIN_ID: u8 = 102;
	//pub(crate) const ERR_INVALID_EVENT_HASH: u8 = 103;
	pub(crate) const ERR_INVALID_TESSARACT_HASH: u8 = 104;
	//pub(crate) const ERR_INVALID_TX_HASH_BY_SOURCE: u8 = 105;
	//pub(crate) const ERR_INVALID_TX_HASH_BY_ACCOUNT: u8 = 106;
	//pub(crate) const ERR_INVALID_TX_HASH_BY_TESSARACT: u8 = 107;

	// range: 121 - 140
	pub(crate) const ERR_INVALID_EVENT_PUBLISHER: u8 = 121;
	pub(crate) const ERR_INVALID_EVENT_NAME: u8 = 122;
	pub(crate) const ERR_INVALID_EVENT_CATEGORY: u8 = 123;
	pub(crate) const ERR_INVALID_EVENT_DATA: u8 = 124;
	//pub(crate) const ERR_INVALID_TIMESTAMP: u8 = 125;

	// range: 141 - 160
	//pub(crate) const ERR_INVALID_THRESHOLD_SIGNATURE: u8 = 141;

	// range: 161 - 180
	//pub(crate) const ERR_INVALID_RECEIVER: u8 = 161;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type TimeProvider: UnixTime;
	}

	// Transaction struct
	// https://www.notion.so/Time-chain-Standard-5154e1d1ba6341b494a88e16aa35b522
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(
		PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug, TypeInfo,
	)]
	pub struct Transaction {
		pub header: TxHeader,
		pub body: TxBody,
		pub footer: TxFooter,
	}

	impl MaxEncodedLen for Transaction {
		fn max_encoded_len() -> usize {
			usize::MAX
		}
	}

	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(
		PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug, TypeInfo,
	)]
	pub struct TxHeader {
		pub source_chain_id: u32,
		pub destination_chain_id: u32,
		pub event_hash: H256,
		pub tessaract_hash: H256,
		pub previous_tx_hash_by_publisher: H256,
		pub previous_tx_hash_by_account: H256,
		pub previous_tx_hash_by_tessaract: H256,
	}

	impl MaxEncodedLen for TxHeader {
		fn max_encoded_len() -> usize {
			usize::MAX
		}
	}

	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(
		PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug, TypeInfo,
	)]
	pub struct TxBody {
		pub event_publisher: H256,
		pub event_name: Vec<u8>,
		pub event_category: Vec<u8>,
		pub event_data: Vec<u8>,
		pub timestamp: u128,
	}

	impl MaxEncodedLen for TxBody {
		fn max_encoded_len() -> usize {
			usize::MAX
		}
	}

	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(
		PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug, TypeInfo,
	)]
	pub struct TxFooter {
		pub threshold_signature: H512,
	}

	impl MaxEncodedLen for TxFooter {
		fn max_encoded_len() -> usize {
			usize::MAX
		}
	}

	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(
		PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug, TypeInfo,
	)]
	pub struct PreviousHashes {
		pub hash_list: Vec<H256>,
	}

	impl MaxEncodedLen for PreviousHashes {
		fn max_encoded_len() -> usize {
			usize::MAX
		}
	}

	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(
		PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug, TypeInfo,
	)]
	pub struct TransactionList {
		pub transactions: Vec<Transaction>,
	}
	impl MaxEncodedLen for TransactionList {
		fn max_encoded_len() -> usize {
			usize::MAX
		}
	}

	// Transaction structure wrapped inside Tx
	// Storing the transaction directly causes DefaultTrait missing error.
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(
		PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug, TypeInfo,
	)]
	pub struct Tx {
		pub transaction: Transaction,
	}
	impl MaxEncodedLen for Tx {
		fn max_encoded_len() -> usize {
			usize::MAX
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_tx_hash_by_event_hash)]
	pub(super) type TxHashByEvent<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, H256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_tx_hash_by_account_hash)]
	pub(super) type TxHashByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, H256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_tx_hash_by_tessaract_hash)]
	pub(super) type TxHashByTessaract<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, H256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_tx_hash_by_publisher_hash)]
	pub(super) type TxHashByPublisher<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, H256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_tx_by_hash)]
	pub(super) type Transactions<T: Config> = StorageMap<_, Blake2_128Concat, H256, Tx, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		EventCreated(T::AccountId, Transaction),
		EventDeleted(T::AccountId, Transaction),
		EventTransfered(T::AccountId, T::AccountId, Transaction),
		EventPublished(Transaction),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		EventAlreadyExists,
		NoSuchEvent,
		NotEventOwner,
		EventNotPublished,
	}

	pub fn hardcode_h256() -> H256 {
		H256([
			0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			0, 0, 0,
		])
	}

	pub fn hardcore_h512() -> H512 {
		H512([
			0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			0, 0, 0, 0, 0, 0,
		])
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100_000)]
		pub fn publish_event(
			origin: OriginFor<T>,
			source_chain_id: u32,
			destination_chain_id: u32,
			tessaract_hash: H256,
			event_publisher: H256,
			event_name: Vec<u8>,
			event_category: Vec<u8>,
			event_data: Vec<u8>,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			let acc_encoded = &account.encode()[..];
			let mut acc_vec = acc_encoded.to_vec();

			let account_hash: H256;

			// Unit tests sends an account id of 8 bytes
			if acc_vec.len() < 32 {
				let mut acc_vec_fix_size = vec![];
				acc_vec_fix_size.resize(32 - acc_vec.len(), 0);
				acc_vec_fix_size.append(&mut acc_vec);
				account_hash = H256::from_slice(&acc_vec_fix_size[..]);
			} else {
				account_hash = H256::from_slice(&acc_encoded[..]);
			}

			let mut transaction = Transaction {
				header: TxHeader {
					source_chain_id,
					destination_chain_id,
					event_hash: hardcode_h256(),
					tessaract_hash,
					previous_tx_hash_by_publisher: hardcode_h256(),
					previous_tx_hash_by_account: hardcode_h256(),
					previous_tx_hash_by_tessaract: hardcode_h256(),
				},
				body: TxBody {
					event_publisher,
					event_name,
					event_category,
					event_data,
					timestamp: T::TimeProvider::now().as_millis(),
				},
				footer: TxFooter { threshold_signature: hardcore_h512() },
			};

			transaction.body.using_encoded(|ref slice| {
				transaction.header.event_hash = BlakeTwo256::hash(&slice);
			});

			// Fetch previous_tx_hash_by_publisher if exists in database
			if TxHashByPublisher::<T>::contains_key(&event_publisher) {
				let tx_hash = TxHashByPublisher::<T>::get(&event_publisher);
				transaction.header.previous_tx_hash_by_publisher = tx_hash;
			}

			// Fetch previous_tx_hash_by_account if exists in database
			if TxHashByAccount::<T>::contains_key(&account_hash) {
				let tx_hash = TxHashByAccount::<T>::get(&account_hash);
				transaction.header.previous_tx_hash_by_account = tx_hash;
			}

			// Fetch previous_tx_hash_by_tessaract if exists in database
			if TxHashByTessaract::<T>::contains_key(&tessaract_hash) {
				let tx_hash = TxHashByTessaract::<T>::get(&tessaract_hash);
				transaction.header.previous_tx_hash_by_tessaract = tx_hash;
			}

			// Calculate tx hash and save it into DB
			transaction.using_encoded(|ref slice| {
				let tx_hash = BlakeTwo256::hash(&slice);

				TxHashByEvent::<T>::insert(&transaction.header.event_hash, &tx_hash);
				TxHashByAccount::<T>::insert(&account_hash, &tx_hash);
				TxHashByTessaract::<T>::insert(&tessaract_hash, &tx_hash);
				TxHashByPublisher::<T>::insert(&event_publisher, &tx_hash);
				Transactions::<T>::insert(&tx_hash, Tx { transaction: transaction.clone() });
			});

			Self::deposit_event(Event::EventPublished(transaction));
			Ok(())
		}
	}

	#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct ValidateTransaction<T: Config + Send + Sync>(sp_std::marker::PhantomData<T>);
	impl<T: Config + Send + Sync> sp_std::fmt::Debug for ValidateTransaction<T> {
		#[cfg(feature = "std")]
		fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
			write!(f, "CheckSpecVersion")
		}

		#[cfg(not(feature = "std"))]
		fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
			Ok(())
		}
	}
	impl<T: Config + Send + Sync> SignedExtension for ValidateTransaction<T>
	where
		<T as frame_system::Config>::Call: IsSubType<Call<T>>,
	{
		type AccountId = T::AccountId;
		type Call = T::Call;
		type AdditionalSigned = ();
		type Pre = ();
		const IDENTIFIER: &'static str = "ValidateTransaction";

		fn validate(
			&self,
			_who: &Self::AccountId,
			call: &Self::Call,
			_info: &DispatchInfoOf<Self::Call>,
			_len: usize,
		) -> TransactionValidity {
			match call.is_sub_type() {
				Some(Call::publish_event {
					source_chain_id,
					destination_chain_id,
					tessaract_hash,
					event_publisher,
					event_name,
					event_category,
					event_data,
				}) => {
					validate_source_chain_id(source_chain_id.clone())?;
					validate_destination_chain_id(destination_chain_id.clone())?;
					validate_tessaract_hash(tessaract_hash.clone())?;
					validate_event_publisher(event_publisher.clone())?;
					validate_event_name(event_name.clone())?;
					validate_event_category(event_category.clone())?;
					validate_event_data(event_data.clone())
				},
				_ => Ok(Default::default()),
			}
		}

		fn pre_dispatch(
			self,
			_who: &Self::AccountId,
			_call: &Self::Call,
			_info: &DispatchInfoOf<Self::Call>,
			_len: usize,
		) -> Result<Self::Pre, TransactionValidityError> {
			Ok(Default::default())
		}

		fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
			Ok(())
		}
	}

	// Transaction validations
	fn validate_source_chain_id(id: u32) -> TransactionValidity {
		if id <= 0 {
			Err(InvalidTransaction::Custom(ERR_INVALID_SOURCE_CHAIN_ID).into())
		} else {
			Ok(Default::default())
		}
	}

	fn validate_destination_chain_id(id: u32) -> TransactionValidity {
		if id <= 0 {
			Err(InvalidTransaction::Custom(ERR_INVALID_DEST_CHAIN_ID).into())
		} else {
			Ok(Default::default())
		}
	}

	fn validate_tessaract_hash(hash: H256) -> TransactionValidity {
		if hash.as_bytes().len() < 32 {
			Err(InvalidTransaction::Custom(ERR_INVALID_TESSARACT_HASH).into())
		} else {
			Ok(Default::default())
		}
	}

	fn validate_event_publisher(hash: H256) -> TransactionValidity {
		if hash.as_bytes().len() < 32 {
			Err(InvalidTransaction::Custom(ERR_INVALID_EVENT_PUBLISHER).into())
		} else {
			Ok(Default::default())
		}
	}

	fn validate_event_name(name: Vec<u8>) -> TransactionValidity {
		if name.len() <= 0 || name.len() > 20 {
			Err(InvalidTransaction::Custom(ERR_INVALID_EVENT_NAME).into())
		} else {
			Ok(Default::default())
		}
	}

	fn validate_event_category(cat: Vec<u8>) -> TransactionValidity {
		if cat.len() <= 0 || cat.len() > 20 {
			Err(InvalidTransaction::Custom(ERR_INVALID_EVENT_CATEGORY).into())
		} else {
			Ok(Default::default())
		}
	}

	fn validate_event_data(data: Vec<u8>) -> TransactionValidity {
		if data.len() > 1024 {
			Err(InvalidTransaction::Custom(ERR_INVALID_EVENT_DATA).into())
		} else {
			Ok(Default::default())
		}
	}
}
