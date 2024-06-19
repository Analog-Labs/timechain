use substrate_wasm_builder::WasmBuilder;

fn main() {
	#[cfg(not(feature = "fast-runtime"))]
	WasmBuilder::init_with_defaults().enable_metadata_hash("TANLOG", 12).build();

	#[cfg(feature = "fast-runtime")]
	WasmBuilder::init_with_defaults().enable_metadata_hash("DANLOG", 12).build();
}
