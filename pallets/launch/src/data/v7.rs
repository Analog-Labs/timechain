//! This file describes the migrations to be run at stage 5.
//!
//! The goal of this migration is to provide partners with liquidity.
use crate::RawEndowmentMigration;

use time_primitives::ANLOG;

// Incentivized testnet validator rewards
pub const DEPOSITS_PRELAUNCH_2: RawEndowmentMigration = &[
	("an7MR9QyibSTcKoniQ1RPjK4thAxhtqBWVgVJqpxxxhbZtSpj", 113_224_700 * ANLOG, None),
	("an7w8NaaeUqMrN33zMLr8aSVYHdzoyGopqwbLuddpxCBnMwV5", 113_224_638 * ANLOG, None),
];
