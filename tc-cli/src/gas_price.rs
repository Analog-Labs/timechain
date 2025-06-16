use crate::config::NetworkConfig;
use crate::env::CoinMarketCap;
use crate::Tc;
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::collections::HashMap;
use time_primitives::NetworkId;

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

fn gas_price(
	usd_src: f64,
	src_decimals: u8,
	usd_dest: f64,
	dest_decimals: u8,
	dest_max_gas_price: u128,
) -> f64 {
	usd_dest / usd_src
		* dest_max_gas_price as f64
		* f64::powi(10., src_decimals as i32 - dest_decimals as i32)
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

	pub fn balance_to_usd(&self, network: NetworkId, balance: u128) -> Result<f64> {
		let token_price = self.config.token_price_usd(network)?;
		let decimals = self.currency(Some(network))?.decimals;
		let factor = 10.0f64.powi(decimals as i32);
		Ok(balance as f64 / factor * token_price)
	}

	/// Calculates destination network gas fee expressed in source network token
	pub fn gas_price(&self, src_network: NetworkId, dest_network: NetworkId) -> Result<f64> {
		let usd_src = self.config.token_price_usd(src_network)?;
		let usd_dest = self.config.token_price_usd(dest_network)?;
		let src_decimals = self.currency(Some(src_network))?.decimals;
		let dest_decimals = self.currency(Some(dest_network))?.decimals;
		let dest_max_gas_price = self.config.network(dest_network)?.max_gas_price;
		Ok(gas_price(usd_src, src_decimals, usd_dest, dest_decimals, dest_max_gas_price))
	}
}
