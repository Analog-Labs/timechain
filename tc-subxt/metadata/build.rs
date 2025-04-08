use std::path::{Path, PathBuf};

fn substitute(path: &str, with: &str) -> String {
	format!(r#"substitute_type(path = "{path}", with = "::subxt::utils::Static<{with}>"),"#)
}

fn derive(path: &Path, module: &str) -> String {
	let simple_types = [
		"time_primitives::dmail::DmailTo",
		"time_primitives::dmail::DmailPath",
		"time_primitives::gmp::GmpMessage",
		"time_primitives::gmp::GatewayOp",
		"time_primitives::gmp::GatewayMessage",
		"time_primitives::gmp::GmpEvent",
		"time_primitives::network::CctpContracts",
		"time_primitives::network::CctpUrl",
		"time_primitives::network::ChainName",
		"time_primitives::network::Network",
		"time_primitives::network::NetworkConfig",
		"time_primitives::shard::Commitment",
		"time_primitives::shard::MemberStatus",
		"time_primitives::shard::ShardStatus",
		"time_primitives::task::Task",
		"time_primitives::task::TaskResult",
		"time_primitives::task::GmpEvents",
		"time_primitives::task::ErrorMsg",
	];
	let others = [
		("sp_core::crypto::AccountId32", "time_primitives::AccountId"),
		("sp_runtime::MultiSigner", "time_primitives::PublicKey"),
	];
	let mut substitutes = String::new();
	for ty in simple_types {
		substitutes.push_str(&substitute(ty, ty));
		substitutes.push('\n');
	}
	for (path, with) in others {
		substitutes.push_str(&substitute(path, with));
		substitutes.push('\n');
	}
	format!(
		r#"#[subxt::subxt(
		runtime_metadata_path = "{}",
		derive_for_all_types = "PartialEq, Clone",
		{substitutes}
	)]
	pub mod {module} {{}}
	"#,
		path.display(),
	)
}

fn timechain(wbuild: &Path) -> String {
	let mainnet = wbuild.join("timechain-runtime").join("timechain_runtime.metadata.scale");
	derive(&mainnet, "timechain")
}

fn main() {
	let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();

	// Try multiple possible paths for the wbuild directory
	let possible_paths = [
		// Standard path
		out_dir.parent().unwrap().parent().unwrap().parent().unwrap().join("wbuild"),
		// CI path with testnet
		out_dir
			.parent()
			.unwrap()
			.parent()
			.unwrap()
			.parent()
			.unwrap()
			.join("testnet")
			.join("wbuild"),
		// Fallback path - try to find it in the workspace root
		PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
			.parent()
			.unwrap()
			.parent()
			.unwrap()
			.join("target")
			.join("wbuild"),
		// CI fallback path
		PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
			.parent()
			.unwrap()
			.parent()
			.unwrap()
			.join("target")
			.join("x86_64-unknown-linux-musl")
			.join("testnet")
			.join("wbuild"),
	];

	// Try each path until we find one that works
	for wbuild in possible_paths.iter() {
		let metadata_path =
			wbuild.join("timechain-runtime").join("timechain_runtime.metadata.scale");
		if metadata_path.exists() {
			println!("Found metadata at: {}", metadata_path.display());
			std::fs::write(out_dir.join("metadata.rs"), timechain(wbuild)).unwrap();
			return;
		}
	}

	// If we get here, we couldn't find the metadata file
	panic!("Could not find timechain_runtime.metadata.scale in any of the expected locations. Make sure to build the runtime first.");
}
