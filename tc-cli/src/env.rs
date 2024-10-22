use anyhow::{Context, Result};

pub struct Mnemonics {
	pub timechain_mnemonic: String,
	pub target_mnemonic: String,
}

impl Mnemonics {
	pub fn from_env() -> Result<Self> {
		Ok(Self {
			timechain_mnemonic: std::env::var("TIMECHAIN_MNEMONIC")
				.context("missing var `TIMECHAIN_MNEMONIC`")?,
			target_mnemonic: std::env::var("TARGET_MNEMONIC")
				.context("missing var `TARGET_MNEMONIC`")?,
		})
	}
}

pub struct Loki {
	pub loki_url: String,
	pub loki_username: String,
	pub loki_password: String,
}

impl Loki {
	pub fn from_env() -> Result<Self> {
		Ok(Self {
			loki_url: std::env::var("LOKI_URL").context("missing var `LOKI_URL`")?,
			loki_username: std::env::var("LOKI_USERNAME").context("missing var `LOKI_USERNAME`")?,
			loki_password: std::env::var("LOKI_PASSWORD").context("missing var `LOKI_PASSWORD`")?,
		})
	}
}

pub struct CoinMarketCap {
	pub token_price_url: String,
	pub token_api_key: String,
}

impl CoinMarketCap {
	pub fn from_env() -> Result<Self> {
		Ok(Self {
			token_price_url: std::env::var("TOKEN_PRICE_URL")
				.context("missing var `TOKEN_PRICE_URL`")?,
			token_api_key: std::env::var("TOKEN_API_KEY").context("missing var `TOKEN_API_KEY`")?,
		})
	}
}
