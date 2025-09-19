use crate::deposits::RawVestedDepositStage;

use time_primitives::{ANLOG, MILLIANLOG};

pub const SEED_SNAPSHOT_12: RawVestedDepositStage = &[
	(
		"an7DT3GVyJuJTddNy4pqVtgHH9zPhqjQyFSD8odDYgrsJZ7xV",
		108_695_652 * ANLOG,
		Some((108_695_652 * ANLOG, 13_749 * MILLIANLOG, 2_008_470)),
	),
	(
		"an7DT3GVyJuJTddNy4pqVtgHH9zPhqjQyFSD8odDYgrsJZ7xV",
		163_043_478 * ANLOG,
		Some((163043478 * ANLOG, 15_467 * MILLIANLOG, 3_326_070)),
	),
	("an7DT3GVyJuJTddNy4pqVtgHH9zPhqjQyFSD8odDYgrsJZ7xV", 271739130 * ANLOG, None),
];
