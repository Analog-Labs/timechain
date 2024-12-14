//! Money matters.

//use polkadot_sdk::*;

pub use time_primitives::currency::*;

pub const TOTAL_ISSUANCE: Balance = 90_570_710 * ANLOG;

pub const TRANSACTION_BYTE_FEE: Balance = MICROANLOG;
pub const STORAGE_BYTE_FEE: Balance = 300 * MILLIANLOG; // Change based on benchmarking

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 750 * MILLIANLOG + (bytes as Balance) * STORAGE_BYTE_FEE
}
