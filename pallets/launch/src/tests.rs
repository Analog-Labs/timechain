use crate::allocation::Allocation;
use crate::deposits::{BalanceOf, CurrencyOf};
use crate::mock::*;
use crate::{ledger::LaunchLedger, Event, Pallet, LAUNCH_LEDGER, LAUNCH_VERSION, STORAGE_VERSION};

use polkadot_sdk::*;

use frame_support::traits::{Currency, StorageVersion, VestingSchedule};

use time_primitives::MILLIANLOG as mANLOG;

/// Current expected on-chain stage version to test
const ON_CHAIN_STAGE: u16 = 53;
/// Wrapped expected on-chain stage version to test
const ON_CHAIN_VERSION: StorageVersion = StorageVersion::new(ON_CHAIN_STAGE);

/// Targeted height at which to execute this migration (to simulate unlocks)
const ON_CHAIN_HEIGHT: u64 = 2_745_000;

/// The number of expected migrations to run and test
const NUM_MIGRATIONS: u16 = LAUNCH_VERSION - ON_CHAIN_STAGE;

/// The number of expected airdrop transfers (will fail in tests)
const NUM_AIRDROP_TRANSFER: usize = 0;

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
		mint_virtual(Allocation::Seed, 1_515_421_307_830 * mANLOG);
		mint_virtual(Allocation::Opportunity1, 170_807_453_140 * mANLOG);
		mint_virtual(Allocation::Private1, 831_031_882_350 * mANLOG);
		mint_virtual(Allocation::Opportunity2, 42_701_863_290 * mANLOG);
		mint_virtual(Allocation::Opportunity3, 53_495_311_080 * mANLOG);
		mint_virtual(Allocation::Opportunity4, 27_242_593_990 * mANLOG);
		mint_virtual(Allocation::Strategic, 200_764_070_180 * mANLOG);
		mint_virtual(Allocation::Team, 1_669_384_055_300 * mANLOG);

		mint_virtual(Allocation::Airdrop, 18_529_097_702_450_211_764);
		mint_virtual(Allocation::Initiatives, 1_063_211_583_000 * mANLOG);
		mint_virtual(Allocation::Ecosystem, 690_706_795_271 * mANLOG);

		// Start new block to collect events
		System::set_block_number(ON_CHAIN_HEIGHT);

		// Ensure ledger can be parsed without error events
		let plan = LaunchLedger::<Test>::compile(LAUNCH_LEDGER)
			.and_then(|p| p.verify())
			.expect("Included launch ledger should always be valid");
		let events = System::read_events_for_pallet::<Event<Test>>();
		for event in events.iter() {
			println!("Compile event: {event:?}");
		}
		assert_eq!(events.len(), 0);

		// Ensure each of the migrations can be run successful
		let _w = plan.run();
		let events = System::read_events_for_pallet::<Event<Test>>();
		for event in events.iter() {
			println!("Runtime event: {event:?}");
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
