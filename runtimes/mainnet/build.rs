#[cfg(feature = "std")]
fn main() {
	use substrate_wasm_builder::WasmBuilder;

	#[cfg(not(feature = "development"))]
	WasmBuilder::init_with_defaults()
		.enable_metadata()
		.enable_metadata_hash("ANLOG", 12)
		.build();

	#[cfg(feature = "development")]
	WasmBuilder::init_with_defaults()
		.enable_metadata()
		.enable_metadata_hash("SANLOG", 12)
		.build();
}

#[cfg(not(feature = "std"))]
fn main() {}
