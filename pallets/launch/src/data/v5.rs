//! This file describes the migrations to be run at stage 1.
//!
//! The goal of this migration is to supply exchanges with the necessary tokens
//! before the official token genesis event.
use crate::RawEndowmentMigration;

use time_primitives::ANLOG;

/// Prelaunch exchange deposits round 2.
pub const DEPOSITS_PRELAUNCH_1: RawEndowmentMigration =
	&[("anAncc6xjYvEFkiab8SJH72ne2Su3TFkN7Gzbfw9XQkthAjfp", 39328063 * ANLOG, None)];
