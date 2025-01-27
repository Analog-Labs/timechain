//! This file describes the migrations to be run at stage 11.
//!
//! The goal of this migration is to provide additional prelaunch funding.
use crate::deposits::RawDepositStage;

use time_primitives::ANLOG;

// Additional prelauchn funding
pub const DEPOSITS_PRELAUNCH_4: RawDepositStage = &[
	("anAGnR5SUY3ZMSo6gnwbeQhNb7wvqo3bdjNE96QooDgqEeLhR", 1_000 * ANLOG, None),
	("an9ee5sqqKNuTvu6aky1odRhqBB2zEK5Ye9MSKDHcvcEqDvVJ", 1_500 * ANLOG, None),
	("an7oFdt6D54m6bobQacCDnxSCUHW2JzaKMyMXcspuMQkeQJcf", 1_000 * ANLOG, None),
	("anB6boBwRwHDBpeG5DbJtpKzwXuBkkjnGUCJp3kwwoj3tYwb6", 700 * ANLOG, None),
	("an78cgK26zRyRAqp9iWCUxeSCDUHPdhgHxSfXoYtP19H2BhdC", 113_200_000 * ANLOG, None),
];
