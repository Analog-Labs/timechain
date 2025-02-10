use crate::deposits::RawVestedDepositStage;

use time_primitives::{ANLOG, MICROANLOG, MILLIANLOG};

pub const DEPOSITS_LAUNCHDAY_3: RawVestedDepositStage = &[
	("5DfW8ATyfuryZYLVhk3qWBVBSMBYLQYFGPETBSJ4NjJt4CBV", 452_899 * ANLOG, None),
	("an7JXHXDwHQNBhZbiv1nDiCMR3TKkhiAF7GtjtKoRJKat4dKS", 2_099_004 * ANLOG, None),
	("an954fjmRgk3zj37qJR28NszR5nCXQuQEtwT4oybwJds6vtvJ", 2_099_004 * ANLOG, None),
	(
		"an8enbFnMrXKYDh1eg5UMVrT6kjMyEYyt9XYv4XqQJpKpJBrq",
		1_811_594 * ANLOG,
		Some((1_539_854_900 * MILLIANLOG, 389_560 * MICROANLOG, 690_870)),
	),
	(
		"anATqEvtBoAX9R9gEmqTUtS9N4CApyPCxnh7Ley48kz6Qu9cB",
		724_638 * ANLOG,
		Some((615_942_300 * MILLIANLOG, 155_820 * MICROANLOG, 690_870)),
	),
	("an87CdWgV8W57ErGhmTchu8ZfvxhofMccQRfe3piHDK3QS5SS", 1_811_594 * ANLOG, None),
	(
		"an5s9VGeqXaDZbx4D4vxtiXT6LdxYTSPBZUww6AeCu1jvQdH7",
		4_528_986 * ANLOG,
		Some((4_076_087_400 * MILLIANLOG, 773_390 * MICROANLOG, 690_870)),
	),
];
