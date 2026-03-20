use crate::deposits::RawDepositStage;

use time_primitives::ANLOG;

pub const OPPORTUNITY4_SNAPSHOT_16: RawVestedDepositStage = &[(
	"an5s9VGeqXaDZbx4D4vxtiXT6LdxYTSPBZUww6AeCu1jvQdH7",
	// total was 4_528_986 * ANLOG
	// 10 % TGE = 452_898.6 ANLOG
	// remaining = 4_076_087.4 ANLOG
	4_076_087_400 * MILLIANLOG,
	// reference PR: 1974
	// 10% TGE was already sent manually. Remaining = 90% = 4,076,087.4 ANLOG
	// Vesting: 12 months linear, start = Opportunity4 TGE block (2,008,470)
	// total_vesting_blocks = 12 * 439,200 = 5,270,400
	// per_block = 4,076,087.4 ANLOG / 5,270,400 = 0.773392 ANLOG
	Some((4_076_087_400 * MILLIANLOG, 773 * MILLIANLOG, 2_008_470)),
)];
