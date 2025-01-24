//! This file describes the migrations to be run at stage 11.
//!
//! The goal of this migration is to provide more tokens to operations.
use crate::deposits::RawDepositStage;

use time_primitives::ANLOG;

// Additional operational funding
pub const DEPOSITS_OPERATIONS: RawDepositStage = &[
	("anAGnR5SUY3ZMSo6gnwbeQhNb7wvqo3bdjNE96QooDgqEeLhR", 1000 * ANLOG, None),
	("an9ee5sqqKNuTvu6aky1odRhqBB2zEK5Ye9MSKDHcvcEqDvVJ", 1500 * ANLOG, None),
];
