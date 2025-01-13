//! This file describes the migrations to be run at stage 2.
//!
//! The goal of this migration to add all the airdrops from the the first
//! incentivized testnet snapshot.

use crate::{RawEndowmentMigration, RawVestingSchedule};

use time_primitives::{ANLOG, MILLIANLOG};

/// Example schedule: One week unlock
const LINEAR_EXAMPLE_SCHEDULE: Option<RawVestingSchedule> =
	Some((1000 * ANLOG, 8 * MILLIANLOG, 20000));

/// Example schedule: One week shelf, one week unlock
const SHELF_EXAMPLE_SCHEDULE: Option<RawVestingSchedule> =
	Some((1000 * ANLOG, 8 * MILLIANLOG, 70000));

/// The first incentivized testnet airdrop snapshot.
/// TODO: This is currently test data
pub const AIRDROP_SNAPSHOT_ONE: RawEndowmentMigration = &[
	("5DwU4bPfAPGsGwhEtgMYBcDK8x9PPAc8e121ey4A1q1rJpWv", 1000 * ANLOG, None),
	("5DHuNHn7SgqQTLtskZGcKxxkHwojdQMQQCQCZZfSKPPLbtbs", 2000 * ANLOG, LINEAR_EXAMPLE_SCHEDULE),
	("5CnfuhTGZMqwmu7PujQbSsR7JVXPQEBtovYqNk91jZG7TDUe", 3000 * ANLOG, SHELF_EXAMPLE_SCHEDULE),
	("5CXHqFnzWdZxwgJV9kdgP5UmPwu9BXc9GRcN9hSbjjKqbrUE", 4000 * ANLOG, LINEAR_EXAMPLE_SCHEDULE),
	("5ERCaeUFe3TCB7ibgaC94DSkkEnYf2kemvqJh3R9ikFyJ93D", 5000 * ANLOG, SHELF_EXAMPLE_SCHEDULE),
	("5Fs7jbCghmRgLvmw9keFmQyc8gUt61kTKoE7kUcQhRuP93gm", 1000 * ANLOG, None),
	("5Dt1mneZXD3VA9wDxCq58pDaojjFLdP8QddrRiYYWeu7zZ2g", 2000 * ANLOG, LINEAR_EXAMPLE_SCHEDULE),
	("5CXSQPkL5UDx4cj67HyNGFC2pJxEBnmMDB9h1TwdqEq73Rn8", 3000 * ANLOG, SHELF_EXAMPLE_SCHEDULE),
	("5EZnaft3brRfadvzs2Rff1VGk6c1tSjaBS9MGMvGZSC2qotL", 4000 * ANLOG, LINEAR_EXAMPLE_SCHEDULE),
	("5H1BawwhSpA2FxFBrTf2VJD9M15xohsjLkceWMNScvxfUVQQ", 5000 * ANLOG, SHELF_EXAMPLE_SCHEDULE),
];
