//! Runtime migration to remove buggy pool members
use polkadot_sdk::*;

use frame_support::{
	ensure,
	traits::{fungible::Inspect, Get},
	weights::Weight,
};
use pallet_nomination_pools::{BondedPools, PoolMembers};
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::Zero;
use sp_runtime::AccountId32;

const CORRUPTED_POOL_MEMBERS: &[&str] = &[
	"an67vKKCtXjWYUaErzKp5E97dDnyt39HNZMwTAReK1PLUVdbA",
	"an9XX2NvQXRvFBwMdNiaaXbAKHN4tA6oi7xGVJbffR84AvcDw",
];

// Type alias for balance type
type BalanceOf<T> = <<T as pallet_nomination_pools::Config>::Currency as Inspect<
	<T as frame_system::Config>::AccountId,
>>::Balance;

/// Helper function to ensure consistency
fn total_balance_members<T: pallet_nomination_pools::Config>() -> BalanceOf<T> {
	PoolMembers::<T>::iter().fold(Default::default(), |acc, (_i, d)| acc + d.total_balance())
}

pub struct RemoveBuggyPoolMembersMigration<T>(sp_std::marker::PhantomData<T>);

impl<T: frame_system::Config + pallet_nomination_pools::Config>
	frame_support::traits::OnRuntimeUpgrade for RemoveBuggyPoolMembersMigration<T>
where
	T::AccountId: From<AccountId32>,
{
	fn on_runtime_upgrade() -> Weight {
		let mut weight: Weight = Weight::zero();

		// Wrap storage migration indside transactions that reverts on failure
		if let Err(error) = frame_support::storage::with_storage_layer(|| {
			let balance_before = total_balance_members::<T>();

			for account in CORRUPTED_POOL_MEMBERS {
				// Attempt to parse address
				let account: T::AccountId = AccountId32::from_ss58check(account)
					.map_err(|_| "Failed to parse address")?
					.into();

				if let Some(member) = PoolMembers::<T>::take(&account) {
					// Track weight of pool members update
					weight += T::DbWeight::get().reads_writes(1, 1);

					// Ensure this is actually an offending account
					ensure!(member.points == Zero::zero(), "Member points not zero");
					ensure!(member.total_balance() == Zero::zero(), "Member points not zero");

					// Update the member_counter in the affected pool
					BondedPools::<T>::mutate(member.pool_id, |maybe_pool| {
						// Some consistency checks
						let pool = maybe_pool.as_mut().ok_or("Member pool missing")?;
						ensure!(pool.member_counter > 0, "Member count missmatch");

						// Decrement member counter
						pool.member_counter -= 1;

						Ok::<(), &str>(())
					})?;

					// Track weight of member counter update
					weight += T::DbWeight::get().reads_writes(1, 1);

					log::info!("Successfully removed pool member: {:?}", account.clone());
				} else {
					log::error!("Failed to locate pool member: {account:?}");
				}
			}

			ensure!(
				total_balance_members::<T>() == balance_before,
				"Total member balance was changed."
			);

			Ok::<(), &str>(())
		}) {
			log::error!("Migration failed: {error}");
		}

		weight
	}
}
