//! This file describes the migrations to be run at stage 5.
//!
//! The goal of this migration is to excute a prelaunch deposit.
#![allow(dead_code)]

use crate::RawEndowmentMigration;

use time_primitives::ANLOG;

// Incentivized testnet validator rewards
pub const DEPOSITS_PRELAUNCH_4: RawEndowmentMigration =
	&[("an78cgK26zRyRAqp9iWCUxeSCDUHPdhgHxSfXoYtP19H2BhdC", 113_200_000 * ANLOG, None)];
