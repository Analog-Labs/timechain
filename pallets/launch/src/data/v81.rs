use crate::deposits::RawVestedDepositStage;

use time_primitives::MILLIANLOG;

pub const OPPORTUNITY4_SNAPSHOT_16: RawVestedDepositStage = &[(
	"an5s9VGeqXaDZbx4D4vxtiXT6LdxYTSPBZUww6AeCu1jvQdH7",
	// total was 4_528_986 * ANLOG
	// 10 % TGE = 452_898.6 ANLOG
	// remaining = 4_076_087.4 ANLOG
	4_076_087_400 * MILLIANLOG,
	// starts from TGE = 0 block

	// total_vesting_blocks = 12 * 439,200(blocks in a month) = 5,270,400
	// per_block = 4,076,087.4 ANLOG / 5,270,400 = 0.773392 ANLOG
	Some((4_076_087_400 * MILLIANLOG, 773 * MILLIANLOG, 0)),
)];
