use std::path::{Path, PathBuf};

fn derive(path: &Path, module: &str) -> String {
	format!(
		r#"#[subxt::subxt(
		runtime_metadata_path = "{}",
		derive_for_all_types = "PartialEq, Clone",
	)]
	pub mod {} {{}}
	"#,
		path.display(),
		module
	)
}

fn main() {
	let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();
	let wbuild = out_dir.parent().unwrap().parent().unwrap().parent().unwrap().join("wbuild");
	let mainnet = wbuild.join("mainnet-runtime").join("mainnet_runtime.metadata.scale");
	let testnet = wbuild.join("testnet-runtime").join("testnet_runtime.metadata.scale");
	let metadata = format!("{}\n{}", derive(&mainnet, "timechain"), derive(&testnet, "testnet"));
	std::fs::write(out_dir.join("metadata.rs"), metadata).unwrap();
}
