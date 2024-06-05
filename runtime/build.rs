use substrate_wasm_builder::WasmBuilder;

fn main() {
	WasmBuilder::new()
		.with_current_project()
		.export_heap_base()
		.import_memory()
		.build();

	WasmBuilder::new()
		.with_current_project()
		.set_file_name("fast_wasm_binary.rs")
		.enable_feature("fast-runtime")
		.export_heap_base()
		.import_memory()
		.build();
}
