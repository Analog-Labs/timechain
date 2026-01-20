use crate::deposits::RawDepositStage;

use time_primitives::{ANLOG, MILLIANLOG};

pub const TEAM_SNAPSHOT_15: RawDepositStage = &[
	("an7dMwRTokPamLyCmHDNtf28veXxq7ZSAWU2r3y4bFQLhybG8", 265_217_391 * ANLOG),
	("an9dLvRPjTbUJB2HiECtTYeRGsfsB23Tk7wnhrM7SfwzSCgRN", 163_043_478 * ANLOG),
	("an9LgWhpjLAUum8gXPBby4R9NRQoBtyp1JdSzycgQio9nMZUN", 10_000_000 * ANLOG),
	// 1458333.33 ANLOG = 1_458_333_330 MILLIANLOG
	("anBBVTzk4oMiokCEh1a76rtpx8ubZZ8sUTLK3hpvSQGamqaVW", 1_458_333_330 * MILLIANLOG),
];
