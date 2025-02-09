use crate::deposits::{BalanceOf, CurrencyOf, RawVirtualSource};
use crate::mock::*;
use crate::{ledger::LaunchLedger, Event, Pallet, LAUNCH_LEDGER, LAUNCH_VERSION, STORAGE_VERSION};

use polkadot_sdk::frame_support::traits::Currency;
use polkadot_sdk::frame_support::traits::StorageVersion;

use time_primitives::ANLOG;

/// Current expected on-chain stage version to test
const ON_CHAIN_STAGE: u16 = 18;
/// Wrapped expected on-chain stage version to test
const ON_CHAIN_VERSION: StorageVersion = StorageVersion::new(ON_CHAIN_STAGE);

/// The number of expected migrations to run and test
const NUM_MIGRATIONS: u16 = LAUNCH_VERSION - ON_CHAIN_STAGE;

fn mint_virtual(source: RawVirtualSource, amount: BalanceOf<Test>) {
	let account = Pallet::<Test>::account_id(source);
	let _ = CurrencyOf::<Test>::deposit_creating(&account, amount);
}

/// Runs and verify current launch plan based on assumed on-chain version
#[test]
fn launch_ledger_validation() {
	let _ = env_logger::builder().is_test(true).try_init();

	new_test_ext().execute_with(|| {
		// Set expected on-chain version as configured above
		ON_CHAIN_VERSION.put::<Pallet<Test>>();

		// Mint necessary virtual funds
		mint_virtual(b"airdrop", 1_336_147_462_613_682_971);
		mint_virtual(b"initiatives", 60_386_473 * ANLOG);
		mint_virtual(b"ecosystem", 3_636_364 * ANLOG);

		// Start new block to collect events
		System::set_block_number(1);

		// Ensure ledger can be parsed without error events
		let plan = LaunchLedger::<Test>::compile(LAUNCH_LEDGER)
			.expect("Included launch ledger should always be valid");
		assert_eq!(System::read_events_for_pallet::<Event::<Test>>().len(), 0);

		// Ensure each of the migrations can be run succesful
		let _w = plan.run();
		let events = System::read_events_for_pallet::<Event<Test>>();

		assert_eq!(events.len(), NUM_MIGRATIONS as usize);
		for event in events.iter() {
			assert!(matches!(event, Event::StageExecuted { version: _, hash: _ }));
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

		for (index, (version, amount, stage)) in LAUNCH_LEDGER.iter().enumerate() {
			assert_eq!(index, *version as usize);
			if stage.is_executable() {
				assert_eq!(stage.sum::<Test>(), *amount);
				stage.check::<Test>();
			}
		}
		assert_eq!(System::read_events_for_pallet::<Event::<Test>>().len(), 0);
	});
}
