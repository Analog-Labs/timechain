use anyhow::Result;

const SI_PREFIX: [(i8, char); 12] = [
	(18, 'E'),
	(15, 'P'),
	(12, 'T'),
	(9, 'G'),
	(6, 'M'),
	(3, 'k'),
	(-3, 'm'),
	(-6, 'u'),
	(-9, 'n'),
	(-12, 'p'),
	(-15, 'f'),
	(-18, 'a'),
];

pub struct BalanceFormatter<'a> {
	decimals: u32,
	symbol: &'a str,
}

impl<'a> BalanceFormatter<'a> {
	pub fn new(decimals: u32, symbol: &'a str) -> Self {
		Self { decimals, symbol }
	}

	pub fn format(&self, balance: u128) -> String {
		if balance == 0 {
			return format!("{} {}", balance, self.symbol);
		};
		let digits = balance.ilog10();
		let rel_digits = digits as i64 - self.decimals as i64;
		let unit = rel_digits / 3 * 3;
		assert!((-18..=18).contains(&unit), "balance out of range {}", unit);
		let unit = unit as i8;
		let prefix = SI_PREFIX.iter().find(|(d, _)| *d == unit).map(|(_, p)| p);

		let base = f64::powi(10., unit as i32 + self.decimals as i32);
		let tokens = balance as f64 / base;
		let tokens_str = format!("{tokens:.3}");
		let tokens_str = tokens_str.trim_end_matches('0');
		let tokens_str = tokens_str.trim_end_matches('.');
		if let Some(prefix) = prefix {
			format!("{tokens_str}{prefix} {}", self.symbol)
		} else {
			format!("{tokens_str} {}", self.symbol)
		}
	}

	pub fn parse(&self, balance: &str) -> Result<u128> {
		let balance = balance.trim().replace('_', "");
		if !balance.contains('.') && !balance.ends_with(&self.symbol) {
			return balance.parse().map_err(|_| anyhow::anyhow!("balance is not a valid u128"));
		}
		let balance = balance.strip_suffix(&self.symbol).unwrap_or(&balance).trim_end();
		let (unit, balance) = SI_PREFIX
			.iter()
			.find(|(_, p)| balance.ends_with(*p))
			.map(|(d, p)| (*d, balance.strip_suffix(*p).unwrap()))
			.unwrap_or((0, balance));
		let balance: f64 =
			balance.parse().map_err(|_| anyhow::anyhow!("balance is not a valid f64"))?;
		let base = f64::powi(10., unit as i32 + self.decimals as i32);
		Ok((balance * base) as u128)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_formatter() -> Result<()> {
		let fmt_cases = [
			(0, "0 ANLG"),
			(1, "1p ANLG"),
			(1_000_000_000_000, "1 ANLG"),
			(10_000_000_000_000, "10 ANLG"),
			(100_000_000_000_000, "100 ANLG"),
			(1_000_000_000_000_000, "1k ANLG"),
			(1_200_000, "1.2u ANLG"),
			(1_200_000_000_000_000, "1.2k ANLG"),
		];
		let parse_cases = [("0", 0), ("1_200_000", 1_200_000), ("1.", 1_000_000_000_000)];
		let fmt = BalanceFormatter::new(12, "ANLG");
		for (n, s) in fmt_cases {
			assert_eq!(fmt.format(n), s);
			assert_eq!(fmt.parse(s).unwrap(), n);
		}
		for (s, n) in parse_cases {
			assert_eq!(fmt.parse(s).unwrap(), n);
		}
		Ok(())
	}
}
