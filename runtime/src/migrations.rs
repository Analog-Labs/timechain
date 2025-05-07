use scale_info::prelude::vec;
use scale_info::prelude::vec::Vec;

use polkadot_sdk::*;

use frame_election_provider_support::Get;
use frame_support::traits::Currency;
use frame_support::traits::ExistenceRequirement;
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::traits::VestingSchedule;
use frame_support::weights::Weight;
use pallet_vesting::Config;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::CheckedConversion;
use sp_runtime::traits::Zero;

use time_primitives::{AccountId, Balance, BlockNumber, MICROANLOG};

use crate::configs::staking::RewardPool;
use crate::{Runtime, RuntimeOrigin};

pub type CurrencyOf<T> = <T as pallet_vesting::Config>::Currency;

pub type BalanceOf<T> =
	<CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const STARTING_BLOCK: BlockNumber = 1_929_156;

const TOTAL_AMOUNT: Balance = 699_999_360_000 * MICROANLOG;

const BOOSTED_STAKERS: &[(&str, Balance, Balance)] = &[
	(
		"an6ddTdnX4ZW8vCW2W7Q7WgriH49FmZzARYkuQwVZ4Ww8PYw3",
		234045440000 * MICROANLOG,
		1160939 * MICROANLOG,
	),
	(
		"an99gmJ83WZAPwjWka8ZFatAF5yLQxugHNH4CqUSfxX8RQxt5",
		163622110000 * MICROANLOG,
		811617 * MICROANLOG,
	),
	(
		"an68TkvfsaaQusstYsQFVqsUprgnRUACH9eFcvFjgCRhd7ZJh",
		117702760000 * MICROANLOG,
		583843 * MICROANLOG,
	),
	(
		"an8DZQ5HC12nbPNMAWQzndXAxeBKcMrL6s7xb2YRuwvRJro4c",
		29037040000 * MICROANLOG,
		144032 * MICROANLOG,
	),
	(
		"an5wbTKCoPtwifT3jr3C628TV1sGvVdqvYQeasDew8C3Nmpab",
		28243980000 * MICROANLOG,
		140099 * MICROANLOG,
	),
	(
		"an8G9FDBB8kATwKniG24HKFEKQGBvCKcJ3sMMafgnFMGm7BiK",
		20508130000 * MICROANLOG,
		101726 * MICROANLOG,
	),
	(
		"anA6e3BTFH2o4HKhrGhQmaTkYPSbttFtar3VtgDmfWpiY5Mha",
		19801590000 * MICROANLOG,
		98222 * MICROANLOG,
	),
	(
		"an9LsG1uEsdWPviRMSdXxwGetGy88Fn67RrV5veD8iVeLxNab",
		17568920000 * MICROANLOG,
		87147 * MICROANLOG,
	),
	(
		"an7Ua3GdG2E8EkSt5u86r97vGZgZGsYEnuv1QGY5xXjQzywSh",
		15940940000 * MICROANLOG,
		79072 * MICROANLOG,
	),
	(
		"anBKj9qrobTSRr8Mwj9pzVbMWhtwAD2UxTcXeEtmw5S9hH9zb",
		15227940000 * MICROANLOG,
		75535 * MICROANLOG,
	),
	(
		"anAbj6KLN5ea4AahcU6HtSdjavHQt361qkFw1djKy7vzVjDKj",
		13289650000 * MICROANLOG,
		65920 * MICROANLOG,
	),
	(
		"an6QMEoLxJx4w9eNFqaPP9eFM6ACBnvAPRnKSqFtYiQ18emkS",
		6744500000 * MICROANLOG,
		33454 * MICROANLOG,
	),
	(
		"an7DUHRzKHJ1kbm9FGFPea1EUCJkikLmCQ7Eoe6pSFCWoDAfb",
		4659540000 * MICROANLOG,
		23112 * MICROANLOG,
	),
	(
		"an9SJzbMpkNgc6xskxEqGZZHV26UPpWidjaaSWJMY7uqK2dbY",
		3373410000 * MICROANLOG,
		16733 * MICROANLOG,
	),
	(
		"an7PmZKNZSiPPmmDa5itqcw2wJGTnh2GxK7d5vmD1JWtA66wC",
		1366640000 * MICROANLOG,
		6778 * MICROANLOG,
	),
	(
		"anAYsieuYVTxDiracTGfHbF1Uouv1NJQGaDY6DauARHE4PTZp",
		1193190000 * MICROANLOG,
		5918 * MICROANLOG,
	),
	(
		"an9fTGuKVbbwBchtVJzmETjKaVNXuNMtMszyeFSK2sJJ2XLns",
		969410000 * MICROANLOG,
		4808 * MICROANLOG,
	),
	(
		"an81p2NrpEssbSD5siZwYKFVyMy7hAYgv2ZhjjbMMMPg5iTGg",
		800800000 * MICROANLOG,
		3972 * MICROANLOG,
	),
	(
		"anA1waWkm4fDvbAg2QsZquDpHoptiCLmDAzJx16otagihf2FT",
		774120000 * MICROANLOG,
		3839 * MICROANLOG,
	),
	(
		"an7yo9FodrFTTFE7Nt1mXDSbimhP2PNHhGWaLgD5sHWmUhUjn",
		730930000 * MICROANLOG,
		3625 * MICROANLOG,
	),
	(
		"anA4uWQxdLoaHdeQH1pubPNS1aVz2JMAfMLiQvqFQ5TgYcPtd",
		624610000 * MICROANLOG,
		3098 * MICROANLOG,
	),
	(
		"anAkPxqXoggo5KmjUsEJGLYw8GLoMiS6fqtybBtERaTRLQDGz",
		611320000 * MICROANLOG,
		3032 * MICROANLOG,
	),
	(
		"anAn9kKTftr8i2e8wZi9pSPvTreipVFvzKMoBYSnNHBtA3p8B",
		598030000 * MICROANLOG,
		2966 * MICROANLOG,
	),
	(
		"an9nndGdNy9xsRAvVQRzGmuE15Ryfnow24vrhXHGhEwMfmtF3",
		581180000 * MICROANLOG,
		2882 * MICROANLOG,
	),
	(
		"an5w9QdTacepSQsvHSRYaYu5AhoKy2FMpheomax3iCCSehWcH",
		465140000 * MICROANLOG,
		2307 * MICROANLOG,
	),
	(
		"an72bhuEPLdYaRso1huBzM4hRvok6HST6mZXDWBTi5r2P2T1W",
		412660000 * MICROANLOG,
		2046 * MICROANLOG,
	),
	("anALWBsdRWSoSiR8Ye2KCESP16cdx6JXKPjfNoycFUGDWVVLY", 175800000 * MICROANLOG, 872 * MICROANLOG),
	("an6DnU595qsf9rnydxKcKsBp5WYoEgukT9mFzQg26xgKYmG2c", 146790000 * MICROANLOG, 728 * MICROANLOG),
	("an7XFwf5ACfGQ9H3GWVbcfP31aiXYm5vVHy2U6LwCVGCQo3pe", 91000000 * MICROANLOG, 451 * MICROANLOG),
	("an9Feve5Jth3J8QztAVWjidHEkNi6ELZqundQ1WJUZhcJMwQD", 86380000 * MICROANLOG, 428 * MICROANLOG),
	("an7SeUhopEofZW9RavaQN7gScst4BMpNroDCgDKKWGBkmbHQj", 65780000 * MICROANLOG, 326 * MICROANLOG),
	("an7P4haQZXTL1nqBWRmMpEDTGMJfjNJqtUmKwWmr9oAY37fe2", 64820000 * MICROANLOG, 321 * MICROANLOG),
	("an6qhHkNWKpfLhDvuPjtkhy4GCLUnCRejHhaXHZepbkZJ8rFe", 52360000 * MICROANLOG, 259 * MICROANLOG),
	("an97f5q5MixAw5EddR9QDkDw4C94KJGdD1nudzBFcfmTEwHEg", 40660000 * MICROANLOG, 201 * MICROANLOG),
	("an6QWDcQM7gZm5KhW2H7bCsgP8XbCG4oN5aq8RddfqXzMxmn5", 37140000 * MICROANLOG, 184 * MICROANLOG),
	("an5rtCHaBH6dxXvbeKwuJTQvR221gyKvDv8Jn8aft12J2NAat", 35190000 * MICROANLOG, 174 * MICROANLOG),
	("an6V8DHqkHM7zYeCqFXQCBU6sHf7SRTwQtN8fQRSkZDRJ55XG", 33160000 * MICROANLOG, 164 * MICROANLOG),
	("an8tpYTb6g42KEBDTM59KsYs2wS1D9tkVSVzGFKHcih88Tqpo", 33160000 * MICROANLOG, 164 * MICROANLOG),
	("an7dPhayLQ93rJArDpzrKvFWFAoQjXj3RMcJimjHv6d9GjsJ6", 29880000 * MICROANLOG, 148 * MICROANLOG),
	("an9fTfFkQvPD2vDMrZsmNMHMnHe3Ui11xHyzvZEb4jj1YA72A", 28570000 * MICROANLOG, 141 * MICROANLOG),
	("an8MKALmGS95fHfkzKbo7xuCVuHjemG29azwhPS2ExKE32U3o", 20800000 * MICROANLOG, 103 * MICROANLOG),
	("anBBn1PEVDn4Mawt9bKUWXVQjQEpBLcsQwpo97pdGwSCcLS2z", 20600000 * MICROANLOG, 102 * MICROANLOG),
	("an99Kvki1S9wT137kKQaJoV38QoFDtC4Ea8MsF699cxoikerQ", 19930000000000, 98859126),
	("an5fhC8scJTXQfE9m91MDHKxRMnPZRjJviU4HNPUyVwriwraZ", 19930000000000, 98859126),
	("an8561FmAscUYj38KAWGhdeNW7N97ed62mjU6Q5sVCouWd3cy", 15960000000000, 79166666),
	("an6ktPyB7WyWXcmt17BR4mjzgr6d39R29UfRTVm3HFKJXZRza", 15750000000000, 78125000),
	("an9WjvxXDKM6uJGKS5oDb6YvDQW8aykdTZJebcUMn8Ubh6Yg5", 15280000000000, 75793650),
	("an9PCWQUqQkDRZ2dRinG6FezM3kTW9r7NwLTxzUtaKvESKzty", 13310000000000, 66021825),
	("anBE8YFfP71p5QBaUYeJ3YJ7eQimzJSvzNkpkRkSoscwNSMbA", 13290000000000, 65922619),
	("an8NzZuJp8JLD5aK41T5iya7ExWBrdJK2KdYtBcxzeRLTPgQJ", 5320000000000, 26388888),
	("anAA3zgFupp4agfuybHrY4hNFPH33fBMnHrDsbhSB1oVv9VkH", 3990000000000, 19791666),
	("an8Uo7DwJWMZLeC8DBQPhyLy7akkzAfUgbbgCrmZXKTFavN3h", 3590000000000, 17807539),
	("an7Q3eYT8k6jGNV9Ho7TT9DLnSXBKGsBHr1cAHdxJhT5A4FDh", 3280000000000, 16269841),
	("an9LE7ivn4BjBZTHVjvGMQarnf5ytJ8K4qdsSKR7u6Nxptp9r", 2660000000000, 13194444),
	("an7r9mUuNpQXfMMP8vcHwuQxLkAfUhoMwC7oGavxV5tbsn9Zj", 2660000000000, 13194444),
	("an6uNBGYtPrAu1riB8U9JnR4LsxYfyStL2JTFxX5nBqfUFeFs", 1660000000000, 8234126),
	("anAWfdggXWB4Gtz3sSUcQPkNXk6DvNcXuVumqnu6NXm4mFHZe", 1660000000000, 8234126),
	("an75SJw5wnp9G6oWpnPLMecBMeq7Au8zBsokkGU2WCVxRYsza", 1590000000000, 7886904),
	("an9kW7dTcPmec1wMY2YA8HfXCiWy9y8Gy38LhBNfN8pR3h81d", 1330000000000, 6597222),
	("an5zZ2bf5HFzjz9vN23hBzvn7QUhSdooQF9y1T47pf9UQUxWC", 1060000000000, 5257936),
	("an9HKsdyHnZjepBZk18XQW6ogMYmyQHiLQkr1hxKv7BSPZxEN", 440000000000, 2182539),
	("anBGyFMKDx5cYRy94PjeaCjac8P6RTtbQWSTzv2ejw22xHke4", 330000000000, 1636904),
	("an8XUJKfhfDpRXP7nKZiWMQFgBN1Habr2FNT8TCPUuth9YoAV", 140000000000, 694444),
	("anA4AYttzrNgmpu8mQbyfyoJocbJPh8dzcgjJmMopkdLCKGuB", 130000000000, 644841),
];

type VestedStaker<A, T> = (A, BalanceOf<T>, BalanceOf<T>);

pub struct BoostedStakerLedger<T: Config>(Vec<VestedStaker<T::AccountId, T>>);

impl<T: Config> BoostedStakerLedger<T>
where
	T::AccountId: From<AccountId>,
	T::RuntimeOrigin: From<RuntimeOrigin>,
	Balance: From<BalanceOf<T>>,
{
	/// Create new deposit migration by parsing and converting raw info
	pub fn parse(data: &[(&str, Balance, Balance)]) -> Self {
		let mut checked = vec![];
		for details in data.iter() {
			if let Some(parsed) = Self::parse_details(details) {
				checked.push(parsed)
			} else {
				log::error!("Invalid boosted staker: {:?}", details.0)
			}
		}
		Self(checked)
	}

	/// Parse an individual entry of a deposit migration
	fn parse_details(details: &(&str, Balance, Balance)) -> Option<VestedStaker<T::AccountId, T>> {
		Some((
			AccountId::from_ss58check(details.0).ok()?.into(),
			BalanceOf::<T>::checked_from(details.1)?,
			BalanceOf::<T>::checked_from(details.2)?,
		))
	}

	/// Compute the total amount of minted tokens in this migration
	pub fn total(&self) -> BalanceOf<T> {
		self.0.iter().fold(Zero::zero(), |acc: BalanceOf<T>, &(_, b, _)| acc + b)
	}

	/// Execute deposits as far as possible, log failed deposit as events
	pub fn execute(self) -> Weight {
		let mut weight = Weight::zero();

		for (target, amount, per_block) in self.0.iter() {
			// Checking if the target is able to receive a vested transfer ...
			weight += T::DbWeight::get().reads(1);

			if pallet_vesting::Pallet::<T>::can_add_vesting_schedule(
				target,
				*amount,
				*per_block,
				STARTING_BLOCK.into(),
			)
			.is_err()
			{
				log::error!("Boosted staker is already vested: {target:?}",);
				continue;
			}

			// ... then attempt to transfer tokens directly from virtual deposit ...
			weight += T::DbWeight::get().reads_writes(3, 2);

			if CurrencyOf::<T>::transfer(
				&RewardPool::account_id().into(),
				target,
				*amount,
				ExistenceRequirement::AllowDeath,
			)
			.is_err()
			{
				log::error!("Reward pool is drained: {target:?}",);
				continue;
			}

			// ... and add vesting schedule at the end
			weight += T::DbWeight::get().reads_writes(1, 3);

			pallet_vesting::Pallet::<T>::add_vesting_schedule(
				target,
				*amount,
				*per_block,
				STARTING_BLOCK.into(),
			)
			.expect("No other vesting schedule exists, as checked above; qed");
		}

		weight
	}
}

pub struct RewardBoostedStakers;
impl OnRuntimeUpgrade for RewardBoostedStakers {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let ledger = BoostedStakerLedger::<Runtime>::parse(BOOSTED_STAKERS);

		if ledger.total() == TOTAL_AMOUNT {
			ledger.execute()
		} else {
			log::error!("Failed to parse boosted stakers: {}", ledger.total());
			Weight::zero()
		}
	}
}
