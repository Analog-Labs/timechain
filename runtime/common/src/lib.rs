//! common types and constants used in both pallet tests and runtime
#![cfg_attr(not(feature = "std"), no_std)]

/// Money matters
pub mod currency {

	pub type Balance = u128;

	pub const TOKEN_DECIMALS: u32 = 8;
	const TOKEN_BASE: u128 = 10;
	pub const ANLOG: Balance = TOKEN_BASE.pow(TOKEN_DECIMALS); // 10^8
	pub const MILLIANLOG: Balance = ANLOG / 1000; // 10^5
	pub const MICROANLOG: Balance = MILLIANLOG / 1000; // 10^2
	pub const TOCK: Balance = MICROANLOG / 100; // 1

	pub const TRANSACTION_BYTE_FEE: Balance = 1 * MICROANLOG;
	pub const STORAGE_BYTE_FEE: Balance = 6 * MILLIANLOG;
	pub const WEIGHT_FEE: Balance = 1 * TOCK;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * MICROANLOG + (bytes as Balance) * STORAGE_BYTE_FEE
	}
}
