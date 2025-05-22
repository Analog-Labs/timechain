use crate::deposits::RawVestedDepositStage;

use time_primitives::{ANLOG, MICROANLOG, MILLIANLOG};

pub const ECOSYSTEM_SNAPSHOT_3: RawVestedDepositStage = &[
	(
		"an7ka457gnneS9ppKQtBBLR1RpK9wP99xmz9Pv7xRgqA4QQdx",
		452_899 * ANLOG,
		Some((384_964 * ANLOG, 97_390 * MICROANLOG, 690_870)),
	),
	("an7yo9FodrFTTFE7Nt1mXDSbimhP2PNHhGWaLgD5sHWmUhUjn", 168_055_533 * MILLIANLOG, None),
];
