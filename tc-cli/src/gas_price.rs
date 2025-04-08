use crate::config::NetworkConfig;
use crate::env::CoinMarketCap;
use crate::Tc;
use anyhow::{Context, Result};
use num_bigint::{BigInt, BigUint};
use num_rational::Ratio;
use num_traits::Signed;
use num_traits::{identities::Zero, pow};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::collections::HashMap;
use time_primitives::NetworkId;
use time_primitives::U256;

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

fn bigint_log10(n: &BigUint) -> f64 {
	let n_str = n.to_string();
	let num_digits = n_str.len();
	let most_significant_digit = &n_str[0..1].parse::<f64>().unwrap();
	(num_digits as f64 - 1.0) + most_significant_digit.log10()
}

fn to_fixed(n: Ratio<BigUint>, precision: Option<usize>) -> String {
	let value = n.to_integer();
	let mut fract = n.fract();

	let precision = match precision {
		Some(p) => p,
		None => {
			if fract.is_zero() {
				0
			} else {
				let denominator = n.denom();
				let log_value = bigint_log10(denominator);
				log_value.ceil() as usize + 1
			}
		},
	};

	if precision == 0 {
		return format!("{}", value);
	}

	let mut result = format!("{}", value);
	result.push('.');

	for _ in 0..precision {
		fract *= Ratio::from_integer(10u32.into());
		let int_part = fract.to_integer();
		result.push_str(&format!("{}", int_part));
		fract -= Ratio::from_integer(int_part);
	}

	result
}

fn compute_src_wei_per_dst_gas_rate(
	src_usd_price: Ratio<BigUint>,
	src_decimals: u32,
	dst_usd_price: Ratio<BigUint>,
	dst_decimals: u32,
	dst_gas_fee: u128,
) -> Ratio<BigUint> {
	let src_usd_per_wei =
		src_usd_price / Ratio::from_integer(pow(BigUint::from(10u32), src_decimals as usize));
	tracing::info!("src usd per wei: {:?}", src_usd_per_wei);
	let dst_usd_per_wei =
		dst_usd_price / Ratio::from_integer(pow(BigUint::from(10u32), dst_decimals as usize));
	tracing::info!("dest usd per wei: {:?}", dst_usd_per_wei);
	let dst_usd_per_gas = dst_usd_per_wei * Ratio::from_integer(BigUint::from(dst_gas_fee));
	tracing::info!("dest usd per gas {:?}: {:?}", dst_gas_fee, dst_usd_per_gas);
	dst_usd_per_gas / src_usd_per_wei
}

fn convert_bigint_ratio_to_biguint(ratio: Ratio<BigInt>) -> Result<Ratio<BigUint>> {
	let (numerator, denominator) = ratio.into();

	if numerator.is_negative() || denominator.is_negative() {
		anyhow::bail!("Cannot convert negative ratio to Uint ratio");
	}

	let numerator_biguint =
		numerator.to_biguint().ok_or(anyhow::anyhow!("Unable to convert numberator"))?;
	let denominator_biguint = denominator
		.to_biguint()
		.ok_or(anyhow::anyhow!("Unable to convert denominator"))?;

	Ok(Ratio::new(numerator_biguint, denominator_biguint))
}

fn convert_bigint_to_u256(value: &BigUint) -> Result<U256> {
	let num_bytes = value.to_bytes_be();
	if num_bytes.len() != 32 {
		anyhow::bail!("Invalid bytes for a u256: {}", num_bytes.len())
	}
	Ok(U256::from_big_endian(&num_bytes))
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
			let symbol = self.currency(Some(*network_id))?.1;
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
		let decimals = self.currency(Some(network))?.0;
		let factor = 10.0f64.powi(decimals as i32);
		Ok(balance as f64 / factor * token_price)
	}

	/// Calculates destination network gas fee expressed in source network token
	pub async fn relative_gas_price(
		&self,
		src_network: NetworkId,
		dest_network: NetworkId,
	) -> Result<(U256, U256)> {
		let src_price = self.config.token_price_usd(src_network)?;
		tracing::info!("source price: {:?}", src_price);
		let dest_price = self.config.token_price_usd(dest_network)?;
		tracing::info!("dest price: {:?}", dest_price);
		let dest_gas_fee = self.max_fee_per_gas(dest_network).await?;

		let src_config = self.config.network(src_network)?;
		let src_margin: f64 = src_config.gmp_margin;
		let src_decimals = self.currency(Some(src_network))?.0;

		let dest_decimals = self.currency(Some(dest_network))?.0;

		let src_usd_price =
			Ratio::from_float(src_price).context("Cannot convert float to ratio")?;
		tracing::info!("source ratio: {:?}", src_usd_price);
		let src_usd_price = convert_bigint_ratio_to_biguint(src_usd_price)?;
		tracing::info!("src ratio uint: {:?}", src_usd_price);
		let dest_usd_price =
			Ratio::from_float(dest_price).context("Cannot convert float to ratio")?;
		tracing::info!("dest ratio: {:?}", dest_usd_price);
		let dest_usd_price = convert_bigint_ratio_to_biguint(dest_usd_price)?;
		tracing::info!("dst ratio uint: {:?}", dest_usd_price);

		// Parse the price strings into `Ratio<BigUint>` for arbitrary precision
		let src_margin = Ratio::from_float(src_margin).context("Cannot convert float to ratio")?;
		tracing::info!("src margin: {:?}", src_margin);

		// src to dest relative gas price
		let mut src_to_dest = compute_src_wei_per_dst_gas_rate(
			src_usd_price.clone(),
			src_decimals,
			dest_usd_price.clone(),
			dest_decimals,
			dest_gas_fee,
		);
		tracing::info!("src to dest: {:?}", src_to_dest);

		// Add margin
		src_to_dest += src_to_dest.clone() * convert_bigint_ratio_to_biguint(src_margin.clone())?;
		tracing::info!("src to dest with margin: {:?}", src_to_dest);

		log::info!(
			"relative gas price {src_network} -> {dest_network}: {}",
			to_fixed(src_to_dest.clone(), None),
		);
		let ratio = src_to_dest;
		let numerator = convert_bigint_to_u256(ratio.numer())?;
		let denominator = convert_bigint_to_u256(ratio.denom())?;
		Ok((numerator, denominator))
	}
}
