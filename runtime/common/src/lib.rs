// common types and constants used in both pallet tests and runtime
#![cfg_attr(not(feature = "std"), no_std)]
pub mod constants {

	pub type Balance = u128;
	pub const TOCK: Balance = 100_000_000;
	pub const MICROANLOG: Balance = 1_00 * TOCK;
	pub const MILLIANLOG: Balance = 1_000 * MICROANLOG;
	pub const ANLOG: Balance = 1_000 * MILLIANLOG;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * MICROANLOG + (bytes as Balance) * 6 * MICROANLOG
	}
}
