//! This file describes the migrations to be run at stage 12.
//!
//! The goal of this migration is to end the softlaunch and
//! start the token genesis phase.
use crate::deposits::RawVirtualDepositStage;

use time_primitives::MILLIANLOG as mANLOG;

// Token Genesis Event Allocations
pub const DEPOSITS_TOKEN_GENESIS_EVENT: RawVirtualDepositStage = &[
	(
		b"seed",
		2_116_870_581_830 * mANLOG,
		Some((2_116_870_581_830 * mANLOG, 178_512 * mANLOG, 4_586_070)),
	),
	(
		b"opportunity1",
		170_807_453_140 * mANLOG,
		Some((170_807_453_140 * mANLOG, 16_204 * mANLOG, 3_268_470)),
	),
	(
		b"private1",
		914_546_375_350 * mANLOG,
		Some((914_546_375_350 * mANLOG, 86_762 * mANLOG, 3_268_470)),
	),
	(
		b"opportunity2",
		42_701_863_290 * mANLOG,
		Some((42_701_863_290 * mANLOG, 4_051 * mANLOG, 3_268_470)),
	),
	(
		b"opportunity3",
		53_495_311_080 * mANLOG,
		Some((53_495_311_080 * mANLOG, 6_766 * mANLOG, 3_268_470)),
	),
	(
		b"opportunity4",
		44_418_704_640 * mANLOG,
		Some((44_418_704_640 * mANLOG, 5_618 * mANLOG, 1_950_870)),
	),
	(
		b"strategic",
		376_857_707_180 * mANLOG,
		Some((376_857_707_180 * mANLOG, 47_669 * mANLOG, 1_950_870)),
	),
	(
		b"team",
		1_714_673_910_300 * mANLOG,
		Some((1_714_673_910_300 * mANLOG, 108_446 * mANLOG, 4_586_070)),
	),
	(b"airdrop", 21_067_705_000 * mANLOG, None),
	(
		b"initiatives",
		1_811_594_200_000 * mANLOG,
		Some((1_449_275_360_000 * mANLOG, 68_745 * mANLOG, 633_270)),
	),
	(
		b"ecosystem",
		899_917_179_190 * mANLOG,
		Some((679_553_171_595 * mANLOG, 32_234 * mANLOG, 633_270)),
	),
];
