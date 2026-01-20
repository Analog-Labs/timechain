use crate::deposits::RawVestedDepositStage;

use time_primitives::{ANLOG, MILLIANLOG};

pub const PRIVATE1_SNAPSHOT_15: RawVestedDepositStage =
	// TODO fix None to custom vesting schedule
	&[(
		"an6iDdDXjBqfCHe8krr94PNs2NLyBaoyjkxdBFMH9PQuqUQH5",
		2_368_044 * ANLOG,
		// start block calculation
		// 9 months cliff
		// delay = 690,870 Blocks
		// per_month_blocks = 439,200 Blocks
		// start_block = delay + (9 * per_month_blocks)

		// per block calculation
		// 2 months linear
		// total_vesting_blocks = 2 * per_month_blocks = 878,400 Blocks
		// per_block = 2_368_044 * ANALOG / total_vesting_blocks
		// per_block = 2.6958606557377047 ANLOG
		Some((2_368_044 * ANLOG, 2_695 * MILLIANLOG, 4_643_670)),
	)];
