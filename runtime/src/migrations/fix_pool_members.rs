//! Runtime migration to remove buggy pool members
use frame_support::{traits::Get, weights::Weight};
use pallet_nomination_pools::{BondedPools, PoolMembers};
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

		// Track affected pools to update member_counter
		let mut affected_pools = sp_std::collections::btree_map::BTreeMap::<u32, u32>::new();

		for account in &accounts_to_remove {
			if let Some(member) = PoolMembers::<T>::take(account) {
				weight += T::DbWeight::get().writes(1);
				log::info!("Removed pool member: {account:?}");
				let pool_id = member.pool_id;
				// Increment count of members removed per pool
				affected_pools.entry(pool_id).and_modify(|c| *c += 1).or_insert(1);
			}
		}

		// Update the member_counter for each affected pool
		for (pool_id, count) in affected_pools.iter() {
			BondedPools::<T>::mutate(*pool_id, |maybe_pool| {
				if let Some(pool) = maybe_pool {
					if pool.member_counter >= *count {
						pool.member_counter -= count;
					} else {
						pool.member_counter = 0; // Ensure it doesn't go negative
					}
				}
			});
			weight += T::DbWeight::get().writes(1);
		}

		weight
	}
}
