use crate::config::NetworkConfig;
use crate::env::CoinMarketCap;
use crate::Tc;
use anyhow::{Context, Result};
use csv::{Reader, Writer};
use num_bigint::{BigInt, BigUint};
use num_rational::Ratio;
use num_traits::Signed;
use num_traits::ToPrimitive;
use num_traits::{identities::Zero, pow};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use time_primitives::NetworkId;

#[derive(Clone, Deserialize)]
pub struct TokenPriceData {
	pub data: CryptoData,
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
	pub price: Option<f64>,
}

#[derive(Clone, Deserialize)]
pub struct NetworkPrice {
	pub network_id: NetworkId,
	#[serde(flatten)]
	pub price_data: PriceData,
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
	let dst_usd_per_wei =
		dst_usd_price / Ratio::from_integer(pow(BigUint::from(10u32), dst_decimals as usize));
	let dst_usd_per_gas = dst_usd_per_wei * Ratio::from_integer(BigUint::from(dst_gas_fee));
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

pub fn get_network_price(
	network_prices: &HashMap<NetworkId, PriceData>,
	network_id: &NetworkId,
) -> Result<PriceData> {
	network_prices
		.get(network_id)
		.ok_or_else(|| anyhow::anyhow!("Unable to get network {} from csv", network_id))
		.cloned()
}

pub fn convert_bigint_to_u128(value: &BigUint) -> Result<u128> {
	value
		.to_u128()
		.ok_or_else(|| anyhow::anyhow!("Could not convert bigint to u128"))
}

pub fn read_csv_token_prices(price_path: &Path) -> Result<HashMap<NetworkId, PriceData>> {
	let mut rdr = Reader::from_path(price_path)
		.with_context(|| format!("failed to open {}", price_path.display()))?;

	let mut network_map: HashMap<NetworkId, PriceData> = HashMap::new();
	for result in rdr.deserialize() {
		let record: NetworkPrice = result?;
		network_map.insert(record.network_id, record.price_data);
	}
	Ok(network_map)
}

impl Tc {
	pub async fn fetch_token_prices(&self) -> Result<()> {
		let env = CoinMarketCap::from_env()?;
		let mut header_map = HeaderMap::new();
		header_map.insert(
			"X-CMC_PRO_API_KEY",
			HeaderValue::from_str(&env.token_api_key).expect("Failed to create header value"),
		);
		let price_path = self.config.prices();
		let file = File::create(&price_path)
			.with_context(|| format!("failed to create {}", price_path.display()))?;
		let mut wtr = Writer::from_writer(file);
		wtr.write_record(["network_id", "symbol", "usd_price", "max_fee_per_gas"])?;
		for (network_id, NetworkConfig { coin_id, .. }) in self.config.networks().iter() {
			let symbol = self.currency(Some(*network_id))?.1;
			let token_url = format!("{}{}", env.token_price_url, coin_id);
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
			let base_fee = self.transaction_base_fee(*network_id).await?;

			wtr.write_record(&[
				network_id.to_string(),
				symbol,
				usd_price.to_string(),
				base_fee.to_string(),
			])?;
		}
		wtr.flush()?;
		log::info!("Saved in {:?}", price_path);
		Ok(())
	}

	pub fn read_csv_token_prices(&self) -> Result<HashMap<NetworkId, PriceData>> {
		read_csv_token_prices(&self.config.prices())
	}

	pub fn balance_to_usd(&self, network: NetworkId, balance: u128) -> Result<f64> {
		let prices = self.read_csv_token_prices()?;
		let decimals = self.currency(Some(network))?.0;
		let factor = 10.0f64.powi(decimals as i32);
		let token_price = prices.get(&network).context("no price data")?.usd_price;
		Ok(balance as f64 / factor * token_price)
	}

	pub fn calculate_relative_price(
		&self,
		src_network: NetworkId,
		dest_network: NetworkId,
		src_data: PriceData,
		dest_data: PriceData,
	) -> Result<Ratio<BigUint>> {
		let src_config = self.config.network(src_network)?;
		let src_margin: f64 = src_config.gmp_margin;
		let src_decimals = self.currency(Some(src_network))?.0;

		let dest_decimals = self.currency(Some(dest_network))?.0;
		let dest_gas_fee = dest_data.max_fee_per_gas;

		let src_usd_price =
			Ratio::from_float(src_data.usd_price).context("Cannot convert float to ratio")?;
		let src_usd_price = convert_bigint_ratio_to_biguint(src_usd_price)?;
		let dest_usd_price =
			Ratio::from_float(dest_data.usd_price).context("Cannot convert float to ratio")?;
		let dest_usd_price = convert_bigint_ratio_to_biguint(dest_usd_price)?;

		// Parse the price strings into `Ratio<BigUint>` for arbitrary precision
		let src_margin = Ratio::from_float(src_margin).context("Cannot convert float to ratio")?;

		// src to dest relative gas price
		let mut src_to_dest = compute_src_wei_per_dst_gas_rate(
			src_usd_price.clone(),
			src_decimals,
			dest_usd_price.clone(),
			dest_decimals,
			dest_gas_fee,
		);

		// Add margin
		src_to_dest += src_to_dest.clone() * convert_bigint_ratio_to_biguint(src_margin.clone())?;

		log::info!(
			"relative gas price {src_network} -> {dest_network}: {}",
			to_fixed(src_to_dest.clone(), None),
		);
		Ok(src_to_dest)
	}
}

#[derive(Clone, Deserialize)]
pub struct PriceData {
	pub symbol: String,
	pub usd_price: f64,
	#[serde(deserialize_with = "deserialize_u128")]
	pub max_fee_per_gas: u128,
}

use serde::de::{self, Visitor};
use std::fmt;

struct U128Visitor;

impl<'de> Visitor<'de> for U128Visitor {
	type Value = u128;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("a string or integer representing a u128")
	}

	fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
		value.parse().map_err(de::Error::custom)
	}

	fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
		Ok(value as u128)
	}

	fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
		if value < 0 {
			return Err(de::Error::custom(format!("negative value {} for u128", value)));
		}
		Ok(value as u128)
	}
}

fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
	D: de::Deserializer<'de>,
{
	deserializer.deserialize_any(U128Visitor)
}
