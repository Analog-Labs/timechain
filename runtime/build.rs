use substrate_wasm_builder::WasmBuilder;

fn main() {
	WasmBuilder::init_with_defaults().enable_metadata_hash("TANLOG", 12).build();

	WasmBuilder::init_with_defaults()
		.enable_metadata_hash("DANLOG", 12)
		.set_file_name("fast_wasm_binary.rs")
		.enable_feature("fast-runtime")
		.build();
}
