#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
// pub mod weights;

pub mod weights;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{Currency, ExistenceRequirement::KeepAlive};
	use frame_system::pallet_prelude::*;

	pub type QueryId = u64;
	pub type CollectionId = u64;

	pub(crate) type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	pub trait WeightInfo {
		fn pay_querying() -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Currency: Currency<Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn query_payment)]
	pub type QueryPayment<T: Config> =
		StorageMap<_, Blake2_128Concat, QueryId, (T::AccountId, BalanceOf<T>), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn collection_earnings)]
	pub type CollectionEarnings<T: Config> =
		StorageMap<_, Blake2_128Concat, CollectionId, BalanceOf<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// the record id that uniquely identify
		QueryFeePaid(T::AccountId, QueryId, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Query ID already used
		QueryIdUsed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic for pay querying in Timegraph
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::pay_querying())]
		pub fn pay_querying(
			origin: OriginFor<T>,
			query_id: QueryId,
			collection_id: CollectionId,
			amount: BalanceOf<T>,
			recipient: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!<QueryPayment<T>>::contains_key(query_id), Error::<T>::QueryIdUsed);
			T::Currency::transfer(&who, &recipient, amount, KeepAlive)?;

			QueryPayment::<T>::insert(query_id, (who.clone(), amount));
			CollectionEarnings::<T>::mutate(collection_id, |value| *value += amount);

			Self::deposit_event(Event::QueryFeePaid(who, query_id, amount));

			Ok(())
		}
	}
}
