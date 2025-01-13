//! This file describes the migrations to be run at stage 1.
//!
//! The goal of this migration is to supply exchanges with the necessary tokens
//! before TGE.
use crate::{RawEndowmentMigration, RawVestingSchedule};

use time_primitives::{ANLOG, MILLIANLOG};

/// Example schedule: One week unlock
const LINEAR_EXAMPLE_SCHEDULE: Option<RawVestingSchedule> =
	Some((1000 * ANLOG, 8 * MILLIANLOG, 20000));

/// Example schedule: One week shelf, one week unlock
const SHELF_EXAMPLE_SCHEDULE: Option<RawVestingSchedule> =
	Some((1000 * ANLOG, 8 * MILLIANLOG, 70000));

/// Launch .
/// TODO: This is currently test data
pub const DEPOSITS_PRELAUNCH: RawEndowmentMigration = &[
	("anANQ9g1Jp7hez8CZEp5ksHwnKcsuX3fL2cRtqZfGFzwbk5cY", 2000 * ANLOG, LINEAR_EXAMPLE_SCHEDULE),
	("an8oEsbqAgPhkq7i3ot7p7x2pDms24WFzXnHZnCXbC22MyEmb", 2000 * ANLOG, SHELF_EXAMPLE_SCHEDULE),
	("an8qtwmyD1PbPTj7iLfi97cV73uYQ3p8qxZYuKCphURCtGKHX", 42690 * MILLIANLOG, None),
];
