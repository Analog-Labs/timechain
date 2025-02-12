//! Runtime migration to remove buggy pool members
use frame_support::{traits::Get, weights::Weight};
use pallet_nomination_pools::PoolMembers;
use polkadot_sdk::*;
use sp_core::crypto::Ss58Codec;
use sp_runtime::AccountId32;
use sp_std::vec::Vec;

pub struct RemoveBuggyPoolMembersMigration<T>(sp_std::marker::PhantomData<T>);

impl<T: frame_system::Config + pallet_nomination_pools::Config>
	frame_support::traits::OnRuntimeUpgrade for RemoveBuggyPoolMembersMigration<T>
where
	T::AccountId: From<AccountId32>,
{
	fn on_runtime_upgrade() -> Weight {
		let mut weight: Weight = Weight::zero();

		// Provided SS58 addresses
		let to_remove = [
			"an67vKKCtXjWYUaErzKp5E97dDnyt39HNZMwTAReK1PLUVdbA",
			"an9XX2NvQXRvFBwMdNiaaXbAKHN4tA6oi7xGVJbffR84AvcDw",
			"an9banSywn1r4nUkpVvFaBU5Jn5T32W9YHVvG33qo7AsrC1ci",
			"an9RkL7gJoDerqU71knygQNzPC3Gb1gx5cTJha7h1xinLDWwc",
		];
		let accounts_to_remove = to_remove
			.into_iter()
			.filter_map(|a| {
				if let Ok(acc) = AccountId32::from_ss58check(a) {
					Some(acc.into())
				} else {
					log::info!("FixPoolMembersMigration: parsing failed for: {a:?}");
					None
				}
			})
			.collect::<Vec<T::AccountId>>();

		for account in accounts_to_remove.iter() {
			if PoolMembers::<T>::contains_key(account) {
				PoolMembers::<T>::remove(account);
				log::info!("Removed pool member: {account:?}");
				weight += <T as frame_system::Config>::DbWeight::get().writes(1);
			}
		}

		weight
	}
}
