const DEFAULT_MNEMONIC: &str = "calm trial chicken bachelor where nice hen liberty access differ motion carpet eye strong light";

pub struct Mnemonics {
	pub timechain_mnemonic: String,
	pub target_mnemonic: String,
}

impl Mnemonics {
	pub fn from_env() -> Self {
		Self {
			timechain_mnemonic: std::env::var("TIMECHAIN_MNEMONIC")
				.unwrap_or_else(|_| "//Eve".to_string()),
			target_mnemonic: std::env::var("TARGET_MNEMONIC")
				.unwrap_or_else(|_| DEFAULT_MNEMONIC.to_string()),
		}
	}
}

pub struct Loki {
	pub loki_url: String,
	pub loki_username: String,
	pub loki_password: String,
}

impl Loki {
	pub fn from_env() -> Self {
		Self {
			loki_url: std::env::var("LOKI_URL").unwrap_or_else(|_| "http://127.0.0.1:3100".into()),
			loki_username: std::env::var("LOKI_USERNAME").unwrap_or_default(),
			loki_password: std::env::var("LOKI_PASSWORD").unwrap_or_default(),
		}
	}
}

pub struct CoinGeckoApiKey {
	pub token_api_key: String,
}

impl CoinGeckoApiKey {
	pub fn from_env() -> Self {
		Self {
			token_api_key: std::env::var("TOKEN_API_KEY").unwrap_or_default(),
		}
	}
}
