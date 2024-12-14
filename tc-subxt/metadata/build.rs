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
		"time_primitives::network::ChainName",
		"time_primitives::network::ChainNetwork",
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
	let wbuild = out_dir.parent().unwrap().parent().unwrap().parent().unwrap().join("wbuild");
	std::fs::write(out_dir.join("metadata.rs"), timechain(&wbuild)).unwrap();
}
