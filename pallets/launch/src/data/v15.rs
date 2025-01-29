//! This file describes the migrations to be run at stage 10.
//!
//! The goal of this migration is to mint the testnet
//! validator airdrops.
use crate::airdrops::RawAirdropMintStage;

use time_primitives::ANLOG;

// Incentivized testnet validator rewards
pub const AIRDROPS_VALIDATORS_FAILED: RawAirdropMintStage = &[
	("an8Evn8LD7svf1z8EiMVRNshLajYWEaj7jk6MgAo4jRUPF8Sv", 1_895_325 * ANLOG, None),
	("an6iVGebSszTVJmDeWs54dHExuZV7o3YChT6BfYoKTMKUTW4X", 1_577_242 * ANLOG, None),
	("anB1P8QYwiqH56GAndw995AR9RHDNMHTqWQCPXhWzRZQy4Nk2", 1_357_228 * ANLOG, None),
	("an8UgwNdqGCK8A1DAeZPrUr19YXCUxP6jJ7vG3EmruHAQcq8D", 912_329 * ANLOG, None),
	("an9f4D2BoRHeRDcs8ethZVXgyiR3h3x3iYU78fdEwd3QqdG2C", 160_086 * ANLOG, None),
	("an9KxpzxnvdfoMMmUPxp1Xrif297m9xaWQaHJTYQx17DP5pPu", 160_086 * ANLOG, None),
];
