use crate::{
	deposits::{BalanceOf, CurrencyOf, VestingDetails},
	Config, Event, Pallet, RawVestingSchedule, LOG_TARGET,
};

use polkadot_sdk::*;

use frame_support::traits::Currency;
use frame_support::traits::VestingSchedule;
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::Get;
use sp_runtime::traits::CheckedConversion;
use sp_runtime::{traits::AccountIdConversion, Perbill};

use time_primitives::{Balance, MILLIANLOG as mANLOG};

/// Abstraction for the different allocations
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Allocation {
	Ignore = 0,
	Seed,
	Opportunity1,
	Private1,
	Opportunity2,
	Opportunity3,
	Opportunity4,
	Strategic,
	Team,
	Airdrop,
	Initiatives,
	Ecosystem,
	Bridged,
	#[allow(clippy::upper_case_acronyms)]
	SIZE,
}

impl Allocation {
	/// Total number of allocations
	pub fn num_of() -> usize {
		Self::SIZE as usize
	}

	/// Create allocation from index
	fn from_index(index: usize) -> Self {
		use Allocation::*;

		match index {
			i if i == Seed as usize => Seed,
			i if i == Opportunity1 as usize => Opportunity1,
			i if i == Private1 as usize => Private1,
			i if i == Opportunity2 as usize => Opportunity2,
			i if i == Opportunity3 as usize => Opportunity3,
			i if i == Opportunity4 as usize => Opportunity4,
			i if i == Strategic as usize => Strategic,
			i if i == Team as usize => Team,
			i if i == Airdrop as usize => Airdrop,
			i if i == Initiatives as usize => Initiatives,
			i if i == Ecosystem as usize => Ecosystem,
			i if i == Bridged as usize => Bridged,
			_ => Ignore,
		}
	}

	/// Convert allocation to index
	pub fn index(&self) -> usize {
		// https://doc.rust-lang.org/reference/items/enumerations.html#r-items.enum.discriminant.access-memory
		(unsafe { *(self as *const Self as *const u8) }) as usize
	}

	/// Retrieve sub id used in virtual wallet generation
	pub fn sub_id(&self) -> &'static [u8] {
		use Allocation::*;

		match self {
			Ignore | SIZE => b"",
			Seed => b"seed",
			Opportunity1 => b"opportunity1",
			Private1 => b"private1",
			Opportunity2 => b"opportunity2",
			Opportunity3 => b"opportunity3",
			Opportunity4 => b"opportunity4",
			Strategic => b"strategic",
			Team => b"team",
			Airdrop => b"airdrop",
			Initiatives => b"initiatives",
			Ecosystem => b"ecosystem",
			Bridged => b"bridged-erc20",
		}
	}

	/// Compute account id of virtual wallet tracking issuance
	pub fn account_id<T: Config>(&self) -> T::AccountId {
		T::PalletId::get().into_sub_account_truncating(self.sub_id())
	}

	/// Return total amount issued per allocation
	pub fn total(&self) -> Balance {
		use Allocation::*;

		match self {
			Ignore | SIZE => 0,
			Seed => 2_116_870_581_830 * mANLOG,
			Opportunity1 => 170_807_453_140 * mANLOG,
			Private1 => 914_546_375_350 * mANLOG,
			Opportunity2 => 42_701_863_290 * mANLOG,
			Opportunity3 => 53_495_311_080 * mANLOG,
			Opportunity4 => 44_418_704_640 * mANLOG,
			Strategic => 376_857_707_180 * mANLOG,
			Team => 1_714_673_910_300 * mANLOG,
			Airdrop => 452_898_550_000 * mANLOG,
			Initiatives => 1_811_594_200_000 * mANLOG,
			Ecosystem => 1_359_106_343_190 * mANLOG,
			Bridged => 0,
		}
	}

	/// Return unparsed raw vesting schedule of allocation
	pub fn schedule_raw(&self) -> Option<RawVestingSchedule> {
		use Allocation::*;

		match self {
			Ignore | SIZE => None,
			Seed => Some((2_116_870_581_830 * mANLOG, 178_512 * mANLOG, 4_586_070)),
			Opportunity1 => Some((170_807_453_140 * mANLOG, 16_204 * mANLOG, 3_268_470)),
			Private1 => Some((914_546_375_350 * mANLOG, 86_762 * mANLOG, 3_268_470)),
			Opportunity2 => Some((42_701_863_290 * mANLOG, 4_051 * mANLOG, 3_268_470)),
			Opportunity3 => Some((53_495_311_080 * mANLOG, 6_766 * mANLOG, 3_268_470)),
			Opportunity4 => Some((44_418_704_640 * mANLOG, 5_618 * mANLOG, 1_950_870)),
			Strategic => Some((376_857_707_180 * mANLOG, 47_669 * mANLOG, 1_950_870)),
			Team => Some((1_714_673_910_300 * mANLOG, 108_446 * mANLOG, 4_586_070)),
			Airdrop => None,
			Initiatives => Some((1_086_956_520_000 * mANLOG, 68_745 * mANLOG, 633_270)),
			Ecosystem => Some((679_553_171_595 * mANLOG, 32_234 * mANLOG, 633_270)),
			Bridged => None,
		}
	}

	/// Attempt to parse raw vesting schedule
	pub fn schedule<T: Config>(&self) -> Option<VestingDetails<T>> {
		let (locked, per_block, start) = self.schedule_raw()?;
		Some((
			BalanceOf::<T>::checked_from(locked)?,
			BalanceOf::<T>::checked_from(per_block)?,
			BlockNumberFor::<T>::checked_from(start)?,
		))
	}

	/// Attempt to provide a relative schedule based on size
	pub fn schedule_rel<T: Config>(&self, amount: BalanceOf<T>) -> Option<VestingDetails<T>>
	where
		Balance: From<BalanceOf<T>>,
	{
		let (locked, per_block, start) = self.schedule_raw()?;
		let per_block_rel = Perbill::from_rational(amount.into(), locked) * per_block;
		Some((
			BalanceOf::<T>::checked_from(amount)?,
			BalanceOf::<T>::checked_from(per_block_rel)?,
			BlockNumberFor::<T>::checked_from(start)?,
		))
	}

	/// Change locked token amount
	pub fn set_locked<T: Config>(&self, locked: BalanceOf<T>)
	where
		Balance: From<BalanceOf<T>>,
	{
		let account = self.account_id::<T>();

		// Remove existing schedule
		if pallet_vesting::Pallet::<T>::remove_vesting_schedule(&account, 0).is_err() {
			Pallet::<T>::deposit_event(Event::<T>::DepositSourceMissmatch {
				source: self.sub_id().to_vec(),
			});
			return;
		}

		// Ensure the remaining tokens are locked again
		if let Some(vs) = self.schedule_rel::<T>(locked) {
			pallet_vesting::Pallet::<T>::add_vesting_schedule(&account, vs.0, vs.1, vs.2)
				.expect("Previous vesting schedule war removed; qed");
		} else {
			Pallet::<T>::deposit_event(Event::<T>::DepositSourceMissmatch {
				source: self.sub_id().to_vec(),
			});
		}
	}
}

/// Simple tracker for per-allocation issuance
#[derive(Clone)]
pub struct AllocationTracker([Balance; Allocation::SIZE as usize]);

impl AllocationTracker {
	/// Create new tracker assuming no issuance
	pub fn new() -> Self {
		AllocationTracker([0; Allocation::SIZE as usize])
	}

	/// Track additional issuance for provided source
	pub fn add(&mut self, source: Allocation, amount: Balance) {
		self.0[source.index()] += amount;
	}

	/// Return tracked issuance for a certain allocation
	pub fn per(&self, source: Allocation) -> Balance {
		self.0[source.index()]
	}

	/// Return total tracked issuance
	#[allow(dead_code)]
	pub fn total(&self) -> Balance {
		let mut total = 0;
		for index in 1..Allocation::num_of() {
			total += self.0[index];
		}
		total
	}

	/// Reconsolidate on-chain virtual accounts with current tracked state.
	/// If `exact` is set issuance must be matched exactly, otherwise it should not be exceeded.
	pub fn check<T: Config>(&self, exact: bool) -> bool
	where
		Balance: From<BalanceOf<T>>,
	{
		let mut valid = true;
		for i in 1..Allocation::num_of() {
			let alloc = Allocation::from_index(i);

			// Check if tracked issuance exceeds allocation
			if self.per(alloc) > alloc.total() {
				log::error!(
					target: LOG_TARGET,
					"🧾 Allocation exceeded for {:?}: {:?} > {}", alloc.sub_id(), self.per(alloc), alloc.total()
				);
				valid = false;
			}

			// Check on-chain issuance
			let remaining: Balance = alloc.total() - self.per(alloc);
			let free = CurrencyOf::<T>::free_balance(&alloc.account_id::<T>()).into();
			if exact && remaining != free {
				log::error!(
					target: LOG_TARGET,
					"🧾 Allocation missmatch for {:?}: {} != {}", alloc.sub_id(), remaining, free
				);
				valid = false;
			} else if !exact && remaining > free {
				log::error!(
					target: LOG_TARGET,
					"🧾 Allocation exceeded for {:?}: {} > {}", alloc.sub_id(), remaining, free
				);
				valid = false;
			}
		}
		valid
	}
}
