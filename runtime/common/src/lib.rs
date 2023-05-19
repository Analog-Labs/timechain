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

	pub const UNITS: Balance = 1_000_000_000_000;
	pub const CENTS: Balance = UNITS / 30_000;
	pub const GRAND: Balance = CENTS * 100_000;
	pub const MILLICENTS: Balance = CENTS / 1_000;
	/// The existential deposit.
	pub const EXISTENTIAL_DEPOSIT: Balance = 1 * CENTS;

	pub const fn deposit_1(items: u32, bytes: u32) -> Balance {
		items as Balance * 2_000 * CENTS + (bytes as Balance) * 100 * MILLICENTS
	}
}

#[macro_export]
macro_rules! prod_or_fast {
	($prod:expr, $test:expr) => {
		if cfg!(feature = "fast-runtime") {
			$test
		} else {
			$prod
		}
	};
	($prod:expr, $test:expr, $env:expr) => {
		if cfg!(feature = "fast-runtime") {
			core::option_env!($env).map(|s| s.parse().ok()).flatten().unwrap_or($test)
		} else {
			$prod
		}
	};
}
