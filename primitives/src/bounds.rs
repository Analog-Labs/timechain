use polkadot_sdk::sp_runtime::Perbill;

#[derive(Debug)]
pub struct Bounds {
	pub tasks: Perbill,
	pub batches: Perbill,
	pub dkgs: Perbill,
	pub heartbeats: Perbill,
	pub elections: Perbill,
}

pub const ON_INITIALIZE_BOUNDS: Bounds = Bounds {
	tasks: Perbill::from_percent(40),
	batches: Perbill::from_percent(20),
	dkgs: Perbill::from_percent(10),
	heartbeats: Perbill::from_percent(10),
	elections: Perbill::from_percent(20),
};

#[test]
fn validate_on_initialize_bounds() {
	use polkadot_sdk::sp_runtime::Saturating;
	assert_eq!(
		ON_INITIALIZE_BOUNDS
			.tasks
			.saturating_add(ON_INITIALIZE_BOUNDS.batches)
			.saturating_add(ON_INITIALIZE_BOUNDS.dkgs)
			.saturating_add(ON_INITIALIZE_BOUNDS.heartbeats)
			.saturating_add(ON_INITIALIZE_BOUNDS.elections),
		Perbill::from_percent(100),
		"Bounds {:?} > 100%",
		ON_INITIALIZE_BOUNDS
	);
}
