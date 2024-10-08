use crate::Tc;
use anyhow::Result;
use num_bigint::{BigInt, BigUint};
use num_rational::Ratio;
use num_traits::Signed;
use num_traits::{identities::Zero, pow};
use serde::Deserialize;
const GWEI: u64 = 1_000_000_000;
const AVG_GMP_GAS_COST: u64 = 88_000;

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
				let precision = log_value.ceil() as usize + 1;
				precision
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
	dst_gas_fee: u64,
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

impl Tc {
	pub fn calculate_relative_price(&self) -> Result<()> {
		//sepolia stuff

		//TODO take as param
		let sepolia_usd_price: f64 = 2663.240;
		//TODO read from config
		let sepolia_margin: f64 = 0.0;
		//TODO read from config
		let sepolia_decimals: u32 = 18;
		//TODO read from connector
		let sepolia_gas_fee: u64 = (1.4 * GWEI as f64) as u64;

		//shibuya stuff
		let shibuya_usd_price: f64 = 0.072960;
		let shibuya_margin: f64 = 0.0;
		//TODO read from config
		let shibuya_decimals: u32 = 18;
		//TODO read from connector
		let shibuya_gas_fee: u64 = 1715 * GWEI;

		let sepolia_usd_price = Ratio::from_float(sepolia_usd_price).unwrap();
		println!("shibuya usd_price: {:?}", sepolia_usd_price);
		let sepolia_usd_price = convert_bigint_ratio_to_biguint(sepolia_usd_price)?;
		let shibuya_usd_price = Ratio::from_float(shibuya_usd_price).unwrap();
		let shibuya_usd_price = convert_bigint_ratio_to_biguint(shibuya_usd_price)?;

		// Parse the price strings into `Ratio<BigUint>` for arbitrary precision
		let sepolia_margin = Ratio::from_float(sepolia_margin).unwrap();
		let shibuya_margin = Ratio::from_float(shibuya_margin).unwrap();

		// Compute Wei prices
		let sepolia_wei_price = sepolia_usd_price.clone()
			/ Ratio::from_integer(pow(BigUint::from(10u32), sepolia_decimals as usize));
		let shibuya_wei_price = shibuya_usd_price.clone()
			/ Ratio::from_integer(pow(BigUint::from(10u32), shibuya_decimals as usize));

		// Compute Gas prices
		let sepolia_gas_price =
			sepolia_wei_price.clone() * Ratio::from_integer(BigUint::from(sepolia_gas_fee));
		let shibuya_gas_price =
			shibuya_wei_price.clone() * Ratio::from_integer(BigUint::from(shibuya_gas_fee));

		// Sepolia to Shibuya relative gas price
		let mut sepolia_to_shibuya = compute_src_wei_per_dst_gas_rate(
			sepolia_usd_price.clone(),
			sepolia_decimals,
			shibuya_usd_price.clone(),
			shibuya_decimals,
			shibuya_gas_fee,
		);

		// Shibuya to Sepolia relative gas price
		let mut shibuya_to_sepolia = compute_src_wei_per_dst_gas_rate(
			shibuya_usd_price.clone(),
			shibuya_decimals,
			sepolia_usd_price.clone(),
			sepolia_decimals,
			sepolia_gas_fee,
		);

		// Print relative gas prices
		println!(" ---- Sepolia Summary ---- ");
		println!("       price: ${}", to_fixed(sepolia_usd_price.clone(), None));
		println!("     gas fee: {} gwei", sepolia_gas_fee);
		println!("   gas price: ${}", to_fixed(sepolia_gas_price.clone(), None));

		println!("\n ---- Shibuya Summary ---- ");
		println!("       price: ${}", to_fixed(shibuya_usd_price.clone(), None));
		println!("     gas fee: {} gwei", shibuya_gas_fee);
		println!("   gas price: ${}", to_fixed(shibuya_gas_price.clone(), None));

		// Add margin
		sepolia_to_shibuya +=
			sepolia_to_shibuya.clone() * convert_bigint_ratio_to_biguint(sepolia_margin.clone())?;
		shibuya_to_sepolia +=
			shibuya_to_sepolia.clone() * convert_bigint_ratio_to_biguint(shibuya_margin.clone())?;

		println!(
			"\nSepolia to Shibuya relative gas price: {}",
			to_fixed(sepolia_to_shibuya.clone(), None)
		);
		println!(
			"Shibuya to Sepolia relative gas price: {}",
			to_fixed(shibuya_to_sepolia.clone(), None)
		);

		let avg_gmp_cost_ratio = Ratio::new(BigUint::from(AVG_GMP_GAS_COST), BigUint::from(1u32));
		let sepolia_to_shibuya_gmp_cost = (avg_gmp_cost_ratio.clone() * sepolia_to_shibuya.clone())
			/ Ratio::from_integer(pow(BigUint::from(10u32), sepolia_decimals as usize));
		let shibuya_to_sepolia_gmp_cost = (avg_gmp_cost_ratio * shibuya_to_sepolia.clone())
			/ Ratio::from_integer(pow(BigUint::from(10u32), sepolia_decimals as usize));

		println!(
			"Sending a msg from sep to shib cost in average: ${} ETH",
			to_fixed(sepolia_to_shibuya_gmp_cost.clone(), None),
		);
		println!(
			"Sending a msg from shib to sep cost in average: ${} ASTR",
			to_fixed(shibuya_to_sepolia_gmp_cost.clone(), None),
		);

		println!(
			"Sepolia to Shibuya relative gas price (rational): {}/{}",
			sepolia_to_shibuya.numer(),
			sepolia_to_shibuya.denom()
		);
		println!(
			"Sepolia to Shibuya relative gas price (decimal): {}",
			to_fixed(sepolia_to_shibuya, None)
		);
		println!("Average GMP fee: {} wei", to_fixed(sepolia_to_shibuya_gmp_cost, None));
		println!(
			"Shibuya to Sepolia relative gas price (rational): {}/{}",
			shibuya_to_sepolia.numer(),
			shibuya_to_sepolia.denom()
		);
		println!(
			"Shibuya to Sepolia relative gas price (decimal): {}",
			to_fixed(shibuya_to_sepolia, None)
		);
		println!("Average GMP fee: {} wei", to_fixed(shibuya_to_sepolia_gmp_cost, None));
		Ok(())
	}
}
