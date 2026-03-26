use crate::allocation::Allocation;
use crate::mock::*;
use crate::{ledger::LaunchLedger, Event, Pallet, LAUNCH_LEDGER, LAUNCH_VERSION, STORAGE_VERSION};

use polkadot_sdk::*;

use frame_support::traits::StorageVersion;

/// Current expected on-chain stage version to test
const ON_CHAIN_STAGE: u16 = 80;
/// Wrapped expected on-chain stage version to test
const ON_CHAIN_VERSION: StorageVersion = StorageVersion::new(ON_CHAIN_STAGE);

/// Targeted height at which to execute this migration (to simulate unlocks)
const ON_CHAIN_HEIGHT: u64 = 4_235_000;

/// The number of expected migrations to run and test
const NUM_MIGRATIONS: u16 = LAUNCH_VERSION - ON_CHAIN_STAGE;

/// The number of expected airdrop transfers (will fail in tests)
const NUM_AIRDROP_TRANSFER: usize = 0;

/// Runs and verify current launch plan based on assumed on-chain version
#[test]
fn launch_ledger_validation() {
	let _ = env_logger::builder().is_test(true).try_init();

	new_test_ext().execute_with(|| {
		// Set expected on-chain version as configured above
		ON_CHAIN_VERSION.put::<Pallet<Test>>();

		// Start new block to collect events
		System::set_block_number(ON_CHAIN_HEIGHT);

		// Parse launch ledger
		let plan = LaunchLedger::<Test>::compile(LAUNCH_LEDGER)
			.expect("Included launch ledger should always be valid");

		// Mint on-chain virtual wallet
		plan.to_genesis();

		// Ensure ledger was parsed without error events
		let plan = plan.verify().expect("Included launch ledger can be verified");
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
