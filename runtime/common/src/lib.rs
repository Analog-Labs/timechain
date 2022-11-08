// common types and constants used in both pallet tests and runtime
#![cfg_attr(not(feature = "std"), no_std)]
pub mod constants {

	pub type Balance = u128;

	pub const TOKEN_DECIMALS: u32 = 8;
	const TOKEN_BASE: u128 = 10;
	pub const ANLOG: Balance = TOKEN_BASE.pow(TOKEN_DECIMALS);
	pub const MILLIANLOG: Balance = ANLOG / 1000;
	pub const MICROANLOG: Balance = MILLIANLOG / 1000;
	pub const TOCK: Balance = MICROANLOG / 100;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * MICROANLOG + (bytes as Balance) * 6 * MICROANLOG
	}
}
