//! Runtime migration to remove buggy pool members
use frame_support::{
	traits::{fungible::Inspect, Get},
	weights::Weight,
};
use pallet_nomination_pools::{BondedPools, PoolMembers, TotalValueLocked};
use polkadot_sdk::*;
use sp_core::crypto::Ss58Codec;
use sp_runtime::{traits::Zero, AccountId32};
use sp_std::vec::Vec;

// Type alias for balance type
type BalanceOf<T> = <<T as pallet_nomination_pools::Config>::Currency as Inspect<
	<T as frame_system::Config>::AccountId,
>>::Balance;

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
		let mut affected_pools =
			sp_std::collections::btree_map::BTreeMap::<u32, (u32, BalanceOf<T>)>::new();

		for account in &accounts_to_remove {
			if let Some(member) = PoolMembers::<T>::take(account) {
				log::info!("Removed pool member: {account:?}");
				weight += T::DbWeight::get().writes(1);
				let pool_id = member.pool_id; // Extract the pool ID correctly
				let stake_amount = member.points; // Get the stake amount in raw balance

				// Track both removed count and removed stake for each pool
				affected_pools
					.entry(pool_id)
					.and_modify(|(count, stake)| {
						*count += 1;
						*stake += stake_amount;
					})
					.or_insert((1, stake_amount));
				weight += T::DbWeight::get().writes(1);
			}
		}

		// Update the member_counter and TVL for each affected pool
		for (pool_id, (removed_members, removed_stake)) in affected_pools.iter() {
			BondedPools::<T>::mutate(*pool_id, |maybe_pool| {
				if let Some(pool) = maybe_pool {
					// Decrement member counter correctly by the number of removed members
					if pool.member_counter >= *removed_members {
						pool.member_counter -= removed_members;
					} else {
						pool.member_counter = 0;
					}

					// Ensure TVL does not go negative
					if pool.points >= *removed_stake {
						pool.points -= *removed_stake;
					} else {
						pool.points = Zero::zero();
					}
					TotalValueLocked::<T>::mutate(|x| *x -= *removed_stake);
				}
			});
			weight += T::DbWeight::get().writes(1);
		}

		weight
	}
}
