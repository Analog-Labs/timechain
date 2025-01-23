//! This file describes the migrations to be run at stage 6.
//!
//! The goal of this migration is to end the softlaunch and
//! start the token genesis phase.
#![allow(dead_code)]

use crate::RawEndowmentMigration;

// Incentivized testnet validator rewards
pub const AIRDROPS_VALIDATORS: RawEndowmentMigration = &[];
