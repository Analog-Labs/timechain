//! Build with runtime with the correct token symbol based on features

#[cfg(feature = "std")]
fn main() {
	use substrate_wasm_builder::WasmBuilder;

	// Build runtime to be used on live chain
	#[cfg(not(feature = "develop"))]
	{
		// Build mainnet runtime (default)
		#[cfg(not(feature = "testnet"))]
		WasmBuilder::init_with_defaults()
			.enable_metadata()
			.enable_metadata_hash("ANLOG", 12)
			.build();

		// Build testnet runtime
		#[cfg(feature = "testnet")]
		WasmBuilder::init_with_defaults()
			.enable_metadata()
			.enable_metadata_hash("TANLOG", 12)
			.build();
	}

	// Build develop runtime
	#[cfg(feature = "develop")]
	WasmBuilder::init_with_defaults()
		.enable_metadata()
		.enable_metadata_hash("DANLOG", 12)
		.build();
}

#[cfg(not(feature = "std"))]
fn main() {}
