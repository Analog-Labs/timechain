//! This file describes the migrations to be run at stage 1.
//!
//! The goal of this migration is to supply exchanges with the necessary tokens
//! before the official token genesis event.
use crate::RawEndowmentMigration;

use time_primitives::ANLOG;

/// Prelaunch exchange deposits.
pub const DEPOSITS_PRELAUNCH: RawEndowmentMigration = &[
	("anACAutxx15HrcAKf5UcV4dBKHMiTLWwqWEasemR5UJXR9nah", 39328100 * ANLOG, None),
	("anAncc6xjYvEFkiab8SJH72ne2Su3TFkN7Gzbfw9XQkthAjfp", 39328063 * ANLOG, None),
	("an8YSmkWinrh8HVE6k8691GJPqTfvz7gjBtkAgGfd6PhSfuYT", 13636400 * ANLOG, None),
];
