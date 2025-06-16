use crate::config::NetworkConfig;
use crate::env::CoinMarketCap;
use crate::Tc;
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Deserialize)]
struct TokenPriceData {
	pub data: CryptoData,
}

#[derive(Clone, Deserialize)]
struct CryptoData {
	pub symbol: String,
	pub quote: Quote,
}

#[derive(Clone, Deserialize)]
struct Quote {
	#[serde(rename = "USD")]
	pub usd: PriceInfo,
}

#[derive(Clone, Deserialize)]
struct PriceInfo {
	pub price: Option<f64>,
}

impl Tc {
	pub async fn fetch_token_prices(&mut self) -> Result<()> {
		let env = CoinMarketCap::from_env();
		let mut header_map = HeaderMap::new();
		header_map.insert(
			"X-CMC_PRO_API_KEY",
			HeaderValue::from_str(&env.token_api_key).expect("Failed to create header value"),
		);
		let mut prices = HashMap::new();
		for (network_id, NetworkConfig { coin_id, .. }) in self.config.networks().iter() {
			let symbol = self.currency(Some(*network_id))?.symbol;
			let token_url = format!(
				"https://pro-api.coinmarketcap.com/v2/tools/price-conversion?amount=1&id={coin_id}"
			);
			let client = reqwest::Client::new();
			let request = client.get(token_url).headers(header_map.clone()).build()?;
			log::info!("GET {}", request.url());
			let response = client.execute(request).await?;
			if response.status() != 200 {
				anyhow::bail!("{}", response.status());
			}
			let response = response.json::<TokenPriceData>().await?;
			let data = response.data.clone();
			let usd_price = data
				.quote
				.usd
				.price
				.ok_or_else(|| anyhow::anyhow!("Couldnt fetch token price for {}", symbol))?;
			let symbol = data.symbol;
			prices.insert(*network_id, (symbol, usd_price));
		}
		self.config.save_prices(prices)?;
		log::info!("Saved in prices.csv");
		Ok(())
	}
}
