use crate::deposits::RawDepositStage;

use time_primitives::{ANLOG, MILLIANLOG};

pub const TEAM_SNAPSHOT_15: RawDepositStage = &[
	// 1458333.33 ANLOG = 1_458_333_330 MILLIANLOG
	("anBBVTzk4oMiokCEh1a76rtpx8ubZZ8sUTLK3hpvSQGamqaVW", 1_458_333_330 * MILLIANLOG),
];
