//! Base currency config for the timechain ecosystem

/// Balance of an account.
pub type Balance = u128;

/// Number of decimals in the tokens fixed point representation
pub const TOKEN_DECIMALS: u32 = 12;
/// Base of the decimals in the tokens fixed point representation
const TOKEN_BASE: u128 = 10;

/// One Analog Token in fixed point representation
pub const ANLOG: Balance = TOKEN_BASE.pow(TOKEN_DECIMALS); // 10^12

/// One thousandth of an Analog Token in fixed point representation
pub const MILLIANLOG: Balance = ANLOG / 1000; // 10^9
/// One millionth of an Analog Token in fixed point representation
pub const MICROANLOG: Balance = MILLIANLOG / 1000; // 10^6
/// One billionth of an Analog Token in fixed point representation
pub const NANOANLOG: Balance = MICROANLOG / 1000; // 10^3
/// The smallest unit of Analog Token in fixed point representation
pub const TOCK: Balance = NANOANLOG / 1000; // 1

/// Total issuance at genesis
pub const GENESIS_ISSUANCE: Balance = 9_057_971_000 * ANLOG;

/// Somewhat realistic midterm estimate of total issuance
pub const TARGET_ISSUANCE: Balance = 10_000_000_000 * ANLOG;
