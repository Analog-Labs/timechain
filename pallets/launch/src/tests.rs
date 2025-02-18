use crate::allocation::Allocation;
use crate::deposits::{BalanceOf, CurrencyOf};
use crate::mock::*;
use crate::{ledger::LaunchLedger, Event, Pallet, LAUNCH_LEDGER, LAUNCH_VERSION, STORAGE_VERSION};

use polkadot_sdk::*;

use frame_support::traits::{Currency, StorageVersion, VestingSchedule};
//use sp_runtime::traits::CheckedConversion;

use time_primitives::{ANLOG, MILLIANLOG as mANLOG};

/// Current expected on-chain stage version to test
const ON_CHAIN_STAGE: u16 = 26;
/// Wrapped expected on-chain stage version to test
const ON_CHAIN_VERSION: StorageVersion = StorageVersion::new(ON_CHAIN_STAGE);

/// The number of expected migrations to run and test
const NUM_MIGRATIONS: u16 = LAUNCH_VERSION - ON_CHAIN_STAGE;

/// The number of expected airdrop transfers (will fail in tests)
const NUM_AIRDROP_TRANSFER: usize = if LAUNCH_VERSION > 27 { 212 } else { 0 };

fn mint_virtual(source: Allocation, amount: BalanceOf<Test>) {
	let account = source.account_id::<Test>();
	let _ = CurrencyOf::<Test>::deposit_creating(&account, amount);
	if let Some(vs) = source.schedule::<Test>() {
		pallet_vesting::Pallet::<Test>::add_vesting_schedule(&account, vs.0, vs.1, vs.2)
			.expect("No other vesting schedule exists; qed");
	}
}

/// Runs and verify current launch plan based on assumed on-chain version
#[test]
fn launch_ledger_validation() {
	let _ = env_logger::builder().is_test(true).try_init();

	new_test_ext().execute_with(|| {
		// Set expected on-chain version as configured above
		ON_CHAIN_VERSION.put::<Pallet<Test>>();

		// Set expected on-chain funds as currently tracked on the books
		mint_virtual(Allocation::Seed, 2_116_870_581_830 * mANLOG);
		mint_virtual(Allocation::Opportunity1, 170_807_453_140 * mANLOG);
		mint_virtual(Allocation::Private1, 914_546_375_350 * mANLOG);
		mint_virtual(Allocation::Opportunity2, 42_701_863_290 * mANLOG);
		mint_virtual(Allocation::Opportunity3, 53_495_311_080 * mANLOG);
		mint_virtual(Allocation::Opportunity4, 44_418_704_640 * mANLOG);
		mint_virtual(Allocation::Strategic, 376_857_707_180 * mANLOG);
		mint_virtual(Allocation::Team, 1_669_384_055_300 * mANLOG);

		mint_virtual(Allocation::Airdrop, 19_626_240_537_386_317_029);
		mint_virtual(Allocation::Initiatives, 1_501_013_514_500 * mANLOG - 362_318_840 * ANLOG);
		mint_virtual(Allocation::Ecosystem, 708_390_838_154 * mANLOG - 14_449_903_350 * mANLOG);

		// Start new block to collect events
		System::set_block_number(1);

		// Ensure ledger can be parsed without error events
		let plan = LaunchLedger::<Test>::compile(LAUNCH_LEDGER)
			.expect("Included launch ledger should always be valid");
		assert_eq!(System::read_events_for_pallet::<Event::<Test>>().len(), 0);

		// Ensure each of the migrations can be run successful
		let _w = plan.run();
		let events = System::read_events_for_pallet::<Event<Test>>();

		for event in events.iter() {
			println!("{event:?}");
		}
		assert_eq!(events.len(), NUM_AIRDROP_TRANSFER + NUM_MIGRATIONS as usize);
		for event in events.iter() {
			if !matches!(event, Event::StageExecuted { version: _, hash: _ }) {
				// Airdrop transfer generally fail in testing
				assert!(matches!(event, Event::AirdropTransferMissing { from: _ }));
			}
		}

		// TODO: Check weight

		// Ensure update to the expected stage happend
		assert_eq!(StorageVersion::get::<Pallet::<Test>>(), STORAGE_VERSION);
	});
}

/// Verify launch ledger logic independent of default parser
#[test]
fn launch_plan_parsing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		for (index, (version, source, amount, stage)) in LAUNCH_LEDGER.iter().enumerate() {
			assert_eq!(*version as usize, index);
			assert_ne!(*source, Allocation::SIZE);
			if stage.is_executable() {
				assert_eq!(stage.sum::<Test>(), *amount);
				stage.check::<Test>();
			}
		}
		assert_eq!(System::read_events_for_pallet::<Event::<Test>>().len(), 0);
	});
}
