use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct TokenPriceData {
	pub data: Vec<CryptoData>,
}

#[derive(Clone, Deserialize)]
pub struct CryptoData {
	pub symbol: String,
	pub quote: Quote,
}

#[derive(Clone, Deserialize)]
pub struct Quote {
	#[serde(rename = "USD")]
	pub usd: PriceInfo,
}

#[derive(Clone, Deserialize)]
pub struct PriceInfo {
	pub price: f64,
}
