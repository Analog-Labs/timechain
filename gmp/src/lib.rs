use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time_primitives::{IConnect, NetworkId};

pub use gmp_evm::sol::Gateway;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Backend {
	Evm,
	Grpc,
	Rust,
}

impl std::str::FromStr for Backend {
	type Err = anyhow::Error;

	fn from_str(backend: &str) -> Result<Self> {
		Ok(match backend {
			"evm" => Self::Evm,
			"grpc" => Self::Grpc,
			"rust" => Self::Rust,
			_ => anyhow::bail!("unsupported backend"),
		})
	}
}

impl std::fmt::Display for Backend {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let backend = match self {
			Self::Evm => "evm",
			Self::Grpc => "grpc",
			Self::Rust => "rust",
		};
		f.write_str(backend)
	}
}

impl Backend {
	pub async fn chain(self, network: NetworkId, mnemonic: &str) -> Result<Arc<dyn IConnect>> {
		Ok(match self {
			Self::Evm => Arc::new(gmp_evm::Chain::new(network, mnemonic).await?),
			Self::Grpc => Arc::new(gmp_grpc::Chain::new(network, mnemonic)?),
			Self::Rust => Arc::new(gmp_rust::Chain::new(network, mnemonic)),
		})
	}
}
