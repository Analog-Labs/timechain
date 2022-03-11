#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

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

	pub(crate) const ERR_INVALID_EVENT_NAME: u8 = 101;
	pub(crate) const ERR_INVALID_EVENT_CATEGORY: u8 = 102;
	pub(crate) const ERR_INVALID_EVENT_SUBJECT: u8 = 103;
	pub(crate) const ERR_INVALID_EVENT_OBJECT: u8 = 104;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type TimeProvider: UnixTime;
	}

	// Transaction struct
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
		pub event_hash: H256,
		pub previous_hash: H256,
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
		pub event_name: Vec<u8>,
		pub category: Vec<u8>,
		pub subject: H256,
		pub object: H256,
		pub time_attributes: u128,
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
		pub broadcaster_signature: H512,
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

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub(super) type Transactions<T: Config> =
		StorageMap<_, Blake2_128Concat, Transaction, (H256, T::BlockNumber), ValueQuery>;

	#[pallet::storage]
	pub(super) type Transferes<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Transaction, ValueQuery>;

	#[pallet::storage]
	pub(super) type PreviousHash<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, PreviousHashes, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		EventCreated(T::AccountId, Transaction),
		EventDeleted(T::AccountId, Transaction),
		EventTransfered(T::AccountId, T::AccountId, Transaction),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		EventAlreadyExists,
		NoSuchEvent,
		NotEventOwner,
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
		#[pallet::weight(1_1000)]
		pub fn create_event(origin: OriginFor<T>, transaction: Transaction) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Verify that the specified event has not already been created.
			ensure!(!Transactions::<T>::contains_key(&transaction), Error::<T>::EventAlreadyExists);

			// Get the block number from the FRAME System pallet.
			let current_block = <frame_system::Pallet<T>>::block_number();

			// Store the proof with the sender and block number.
			let encoded_account_id = &sender.encode()[..];
			let h256 = &H256::from_slice(encoded_account_id);
			Transactions::<T>::insert(&transaction, (&h256, current_block));

			Self::deposit_event(Event::EventCreated(sender, transaction));
			Ok(())
		}

		#[pallet::weight(1_1000)]
		pub fn delete_event(origin: OriginFor<T>, transaction: Transaction) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let sender_hash = H256::from_slice(&sender.encode()[..]);

			// Verify that the specified event has been created.
			ensure!(Transactions::<T>::contains_key(&transaction), Error::<T>::NoSuchEvent);

			// Get owner of the claim.
			let (owner_hash, _) = Transactions::<T>::get(&transaction);

			// Verify that sender of the current call is the event owner.
			ensure!(sender_hash == owner_hash, Error::<T>::NotEventOwner);

			// Remove claim from storage.
			Transactions::<T>::remove(&transaction);

			Self::deposit_event(Event::EventCreated(sender, transaction));
			Ok(())
		}

		#[pallet::weight(1_1000)]
		pub fn transfer_event(
			origin: OriginFor<T>,
			to: T::AccountId,
			event_name: Vec<u8>,
			event_category: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let mut transaction = Transaction {
				header: TxHeader { event_hash: hardcode_h256(), previous_hash: hardcode_h256() },
				body: TxBody {
					event_name,
					category: event_category,
					subject: H256::from_slice(&sender.encode()[..]),
					object: H256::from_slice(&to.encode()[..]),
					time_attributes: T::TimeProvider::now().as_millis(),
				},
				footer: TxFooter { broadcaster_signature: hardcore_h512() },
			};

			transaction.body.using_encoded(|ref slice| {
				transaction.header.event_hash = BlakeTwo256::hash(&slice);
				// In case it is the first event from sender
				transaction.header.previous_hash = transaction.header.event_hash
			});

			if PreviousHash::<T>::contains_key(&sender) {
				// Get the list of preivious hashes from sender
				let mut previous_hashes = PreviousHash::<T>::get(&sender);

				transaction.header.previous_hash =
					previous_hashes.hash_list.last().unwrap().clone();

				previous_hashes.hash_list.push(transaction.header.event_hash);
				PreviousHash::<T>::insert(&sender, previous_hashes);
			} else {
				// First transaction, create a new hash_list
				let mut previous_hashes = PreviousHashes { hash_list: Vec::new() };
				previous_hashes.hash_list.push(transaction.header.event_hash);

				PreviousHash::<T>::insert(&sender, previous_hashes);
			}

			// Get owner of the claim.
			Transferes::<T>::insert(&to, &transaction);

			Self::deposit_event(Event::EventTransfered(sender, to, transaction));
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
			who: &Self::AccountId,
			call: &Self::Call,
			_info: &DispatchInfoOf<Self::Call>,
			_len: usize,
		) -> TransactionValidity {
			match call.is_sub_type() {
				Some(Call::transfer_event { to, event_name, event_category }) => {
					// More validations comes here...
					validate_event_name(event_name.clone())?;
					validate_event_category(event_category.clone())?;
					validate_subject(H256::from_slice(&who.encode()[..]))?;
					validate_object(H256::from_slice(&to.encode()[..]))
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

	fn validate_subject(subject: H256) -> TransactionValidity {
		if subject.as_bytes().len() <= 0 || subject.as_bytes().len() > 32 {
			Err(InvalidTransaction::Custom(ERR_INVALID_EVENT_SUBJECT).into())
		} else {
			Ok(Default::default())
		}
	}

	fn validate_object(object: H256) -> TransactionValidity {
		if object.as_bytes().len() <= 0 || object.as_bytes().len() > 32 {
			Err(InvalidTransaction::Custom(ERR_INVALID_EVENT_OBJECT).into())
		} else {
			Ok(Default::default())
		}
	}
}
