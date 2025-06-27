use crate::config::NetworkConfig;
use crate::env::CoinGeckoApiKey;
use crate::Tc;
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone, Deserialize, Debug)]
struct CoinGeckoMarketChart {
	pub prices: Vec<Vec<f64>>,
}

impl Tc {
	// computes daily average price
	pub async fn fetch_token_prices(&mut self) -> Result<()> {
		let env = CoinGeckoApiKey::from_env();
		let mut header_map = HeaderMap::new();
		header_map.insert(
			"x-cg-demo-api-key",
			HeaderValue::from_str(&env.token_api_key).expect("Failed to create header value"),
		);
		let mut prices = HashMap::new();
		for (network_id, NetworkConfig { coin_id, .. }) in self.config.networks().iter() {
			let symbol = self.currency(Some(*network_id))?.symbol;
			let token_url = format!(
				"https://api.coingecko.com/api/v3/coins/{coin_id}/market_chart?vs_currency=usd&days=1"
			);
			let client = reqwest::Client::new();
			let request = client.get(token_url).headers(header_map.clone()).build()?;
			log::info!("GET {}", request.url());
			let response = client.execute(request).await?;
			if response.status() != 200 {
				anyhow::bail!("{}", response.status());
			}
			let response = response.json::<CoinGeckoMarketChart>().await?;
			let daily_average = if response.prices.is_empty() {
				anyhow::bail!("No price data available for coin: {}", symbol);
			} else {
				// index 0 is timestamp and 1 is token price
				let price_values: Vec<f64> =
					response.prices.iter().map(|price_point| price_point[1]).collect();
				let sum: f64 = price_values.iter().sum();
				sum / price_values.len() as f64
			};
			prices.insert(*network_id, (symbol, daily_average));
			// limitation of 30 req/min by coingecko
			tokio::time::sleep(Duration::from_secs(3)).await;
		}
		self.config.save_prices(prices)?;
		log::info!("Saved in prices.csv");
		Ok(())
	}
}
