//! Common types and constants used in both runtimes and any pallet
#![cfg_attr(not(feature = "std"), no_std)]

use polkadot_sdk::*;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;
#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;
#[cfg(any(feature = "std", test))]
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use pallet_utility::Call as UtilityCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Weight matters.
pub mod weights;

/// Money matters.
pub mod currency {
	pub use time_primitives::currency::*;

	pub const TOTAL_ISSUANCE: Balance = 90_570_710 * ANLOG;

	pub const TRANSACTION_BYTE_FEE: Balance = MICROANLOG;
	pub const STORAGE_BYTE_FEE: Balance = 300 * MILLIANLOG;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 750 * MILLIANLOG + (bytes as Balance) * STORAGE_BYTE_FEE
	}
}

/// Time.
pub mod time {
	use time_primitives::{BlockNumber, Moment};

	/// Average expected block time that we are targeting.
	pub const MILLISECS_PER_BLOCK: Moment = 6000;

	/// Minimum duration at which blocks will be produced.
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	// These time units are defined in number of blocks.
	pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;
	pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	pub const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

	/// TODO: 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}

/// Shared signing extensions
pub type SignedExtra<Runtime> = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

use time_primitives::{AccountId, Signature};

/// The address format for describing accounts.
type Address = sp_runtime::MultiAddress<AccountId, ()>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic<Runtime, Call> =
	sp_runtime::generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra<Runtime>>;
/// The payload being signed in transactions.
pub type SignedPayload<Runtime, Call> =
	sp_runtime::generic::SignedPayload<Call, SignedExtra<Runtime>>;

/// Shared default babe genesis config
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: time::PRIMARY_PROBABILITY,
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

/// Macro to set a value (e.g. when using the `parameter_types` macro) to either
/// a production value or a development value (in case the `development` feature is
/// selected).
///
/// Usage:
/// ```Rust
/// parameter_types! {
///     pub const VotingPeriod: BlockNumber = prod_or_dev!(7 * DAYS, 1 * MINUTES);
/// }
/// ```
#[macro_export]
macro_rules! prod_or_dev {
	($prod:expr, $fast:expr) => {
		if cfg!(feature = "development") {
			$fast
		} else {
			$prod
		}
	};
}
