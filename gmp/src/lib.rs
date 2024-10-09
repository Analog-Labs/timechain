use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time_primitives::{ConnectorParams, IConnector, IConnectorAdmin, IConnectorBuilder};

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

impl Backend {
	pub async fn connect(&self, params: &ConnectorParams) -> Result<Arc<dyn IConnector>> {
		Ok(match self {
			Self::Evm => Arc::new(gmp_evm::Connector::new(params.clone()).await?),
			Self::Grpc => Arc::new(gmp_grpc::Connector::new(params.clone()).await?),
			Self::Rust => Arc::new(gmp_rust::Connector::new(params.clone()).await?),
		})
	}

	pub async fn connect_admin(
		&self,
		params: &ConnectorParams,
	) -> Result<Arc<dyn IConnectorAdmin>> {
		Ok(match self {
			Self::Evm => Arc::new(gmp_evm::Connector::new(params.clone()).await?),
			Self::Grpc => Arc::new(gmp_grpc::Connector::new(params.clone()).await?),
			Self::Rust => Arc::new(gmp_rust::Connector::new(params.clone()).await?),
		})
	}
}
