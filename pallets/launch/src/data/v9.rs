//! This file describes the migrations to be run at stage 6.
//!
//! The goal of this migration is to end the softlaunch and
//! start the token genesis phase.
#![allow(dead_code)]

use crate::RawEndowmentMigration;

type RawVestingInMonths = (u16, u16);

/// TODO: Needs conversion into block and per-block values
const V_0_6: RawVestingInMonths = (0, 6);
const V_0_9: RawVestingInMonths = (0, 9);
const V_3_18: RawVestingInMonths = (3, 18);
const V_6_18: RawVestingInMonths = (6, 18);
const V_6_24: RawVestingInMonths = (6, 24);
const V_9_27: RawVestingInMonths = (9, 27);
const V_9_36: RawVestingInMonths = (9, 36);

// Incentivized testnet validator rewards
pub const DEPOSITS_TOKEN_GENESIS_EVENT: RawEndowmentMigration = &[];
