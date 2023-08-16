//! common types and constants used in both pallet tests and runtime
#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;

pub mod currency {

	pub type Balance = u128;

	pub const TOKEN_DECIMALS: u32 = 12;
	const TOKEN_BASE: u128 = 10;
	pub const ANLOG: Balance = TOKEN_BASE.pow(TOKEN_DECIMALS); // 10^12
	pub const MILLIANLOG: Balance = ANLOG / 1000; // 10^9
	pub const MICROANLOG: Balance = MILLIANLOG / 1000; // 10^6
	pub const NANOANLOG: Balance = MICROANLOG / 1000; // 10^3
	pub const TOCK: Balance = NANOANLOG / 1000; // 1

	pub const TRANSACTION_BYTE_FEE: Balance = MICROANLOG;
	pub const STORAGE_BYTE_FEE: Balance = 500 * MILLIANLOG;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * MICROANLOG + (bytes as Balance) * STORAGE_BYTE_FEE
	}
}
