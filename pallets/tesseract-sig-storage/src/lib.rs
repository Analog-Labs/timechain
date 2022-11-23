#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
mod types;

pub mod weights;
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;


#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use crate::weights::WeightInfo;
	use sp_std::vec::Vec;
	// use pallet_randomness_collective_flip;
	use crate::types::*;
	use frame_support::traits::Randomness;
	// use sp_std::hash::Hash;
	use sp_io::hashing::blake2_128;
	use sp_runtime::{traits::Hash, RuntimeDebug};
	
	// #[cfg(feature = "std")]
    // use serde::{Deserialize, Serialize};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);
	// #[pallet::config]
    // pub trait Config: frame_system::Config + pallet_randomness_collective_flip::Config {}
    
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type StoreRandomness: Randomness<Self::Hash, Self::BlockNumber>;
	}
	
	#[pallet::storage]
	#[pallet::getter(fn get_nonce)]
	pub(super) type Nonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tesseract_members)]
	pub type TesseractMembers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, TesseractRole, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn signature_store)]
	pub type SignatureStore<T: Config> =
		StorageMap<_, Blake2_128Concat, T::Hash, SignatureData, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn signature_storage)]
	pub type SignatureStoreData<T: Config> =
		StorageMap<_, Blake2_128Concat, T::Hash, SignatureStorage<T::Hash>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event data for stored signature
		/// the signature id that uniquely identify the signature
		SignatureStored(T::Hash),

		/// A tesseract Node has been added as a member with it's role
		TesseractMemberAdded(T::AccountId, TesseractRole),

		/// A tesseract Node has been removed
		TesseractMemberRemoved(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The Tesseract address in not known
		UnknownTesseract,
	}
	
	
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for storing a signature
		#[pallet::weight(
			T::WeightInfo::store_signature()
		)]
		pub fn store_signature(
			origin: OriginFor<T>,
			// signature_key: SignatureKey,
			signature_data: SignatureData,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let random_value  = Self::random_hash(&caller);
			// let random_value  = Self::gen_dna();
			// let random_value = <pallet_randomness_collective_flip::Pallet<T>>::random_seed();
			let signature_key = random_value;
			ensure!(TesseractMembers::<T>::contains_key(caller), Error::<T>::UnknownTesseract);

			<SignatureStore<T>>::insert(signature_key.clone(), signature_data);

			Self::deposit_event(Event::SignatureStored(signature_key));

			Ok(())
		}

		#[pallet::weight(
			T::WeightInfo::store_signature_data()
		)]
		pub fn store_signature_data(
			origin: OriginFor<T>,
			signature_data: SignatureData,
			network_id: Vec<u8>,
			block_height: u64,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			// let random_value = <pallet_randomness_collective_flip::Pallet<T>>::random_seed();
			// let random_value = <pallet_randomness_collective_flip::Pallet<T>>::random(&b"my context"[..]);
			ensure!(TesseractMembers::<T>::contains_key(caller.clone()), Error::<T>::UnknownTesseract);
			let random_value  = Self::random_hash(&caller);
			// let random_value  = Self::gen_dna();
			frame_support::runtime_print!("my value is {}", 3);
			let time_stamp = random_value.clone();
			let hash_key = random_value.clone();// random_value.0.to_string();// as u64;// Lib_Fn::calculate_timeStamp();
			let storage_data = SignatureStorage::new(hash_key.clone(), signature_data.clone(), network_id.to_vec().clone(), block_height, time_stamp);
			
			<SignatureStoreData<T>>::insert(random_value.clone(), storage_data);

			Self::deposit_event(Event::SignatureStored(random_value));

			Ok(())
		}

		/// Extrinsic for adding a node and it's member role
		/// Callable only by root for now
		#[pallet::weight(T::WeightInfo::add_member())]
		pub fn add_member(
			origin: OriginFor<T>,
			account: T::AccountId,
			role: TesseractRole,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			<TesseractMembers<T>>::insert(account.clone(), role.clone());

			Self::deposit_event(Event::TesseractMemberAdded(account, role));

			Ok(())
		}

		/// Extrinsic for adding a node and it's member role
		/// Callable only by root for now
		#[pallet::weight(T::WeightInfo::remove_member())]
		pub fn remove_member(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			let _ = ensure_root(origin)?;

			<TesseractMembers<T>>::remove(account.clone());

			Self::deposit_event(Event::TesseractMemberRemoved(account));

			Ok(())
		}
	}
	impl<T: Config> Pallet<T>{
		pub fn random_hash(sender: &T::AccountId) -> T::Hash{
			let nonce = <Nonce<T>>::get();
			let seed = T::StoreRandomness::random(&b"dna"[..]).0;
			
			T::Hashing::hash_of(&(seed, sender, nonce))
		}

		pub fn random_hash1() -> T::Hash {
			let random_value = T::StoreRandomness::random_seed();
			let nonce = <Nonce<T>>::get();

			// T::Hashing::hash_of(&(random_value, nonce))
			random_value.0
		}

		pub fn gen_dna() -> [u8; 16] {
			let payload = (
				T::StoreRandomness::random(&b"dna"[..]).0,
				<frame_system::Pallet<T>>::block_number(),
			);
			payload.using_encoded(blake2_128)
		}
		pub fn random_hash2() {
			let val = T::StoreRandomness::random(&b"dna"[..]);
			
		}
	}
	// fn calculate_hash<T: Hash>(t: &T) -> u64 {
	// 	let mut s = DefaultHasher::new();
	// 	t.hash(&mut s);
	// 	s.finish()
	// }
	// fn string_gen() -> String {
	// 	let str: String = rand::thread_rng()
    //     .sample_iter(&Alphanumeric)
    //     .take(7)
    //     .map(char::from)
    //     .collect();

	// 	str
	// }
	
}
