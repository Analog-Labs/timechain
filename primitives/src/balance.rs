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
	#[must_use]
	pub const fn new(decimals: u32, symbol: &'a str) -> Self {
		Self { decimals, symbol }
	}

	#[must_use]
	#[allow(clippy::missing_panics_doc)]
	pub fn format(&self, balance: u128) -> String {
		if balance == 0 {
			return format!("{} {}", balance, self.symbol);
		};
		let digits = balance.ilog10();
		let rel_digits = i64::from(digits) - i64::from(self.decimals);
		let unit = rel_digits / 3 * 3;
		assert!((-18..=18).contains(&unit), "balance out of range {unit}");
		#[allow(clippy::cast_possible_truncation)]
		let unit = unit as i8;
		let prefix = SI_PREFIX.iter().find(|(d, _)| *d == unit).map(|(_, p)| p);

		let base = f64::powi(10., i32::from(unit) + i32::try_from(self.decimals).unwrap_or(i32::MAX));
		#[allow(clippy::cast_precision_loss)]
		let tokens = balance as f64 / base;
		let tokens_str = format!("{tokens:.3}");
		let tokens_str = tokens_str.trim_end_matches(|c| c == '0' || c == '.');
		#[allow(clippy::option_if_let_else)]
		if let Some(prefix) = prefix {
			format!("{tokens_str}{prefix} {}", self.symbol)
		} else {
			format!("{tokens_str} {}", self.symbol)
		}
	}

	#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc, clippy::unwrap_used, clippy::cast_lossless, clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]
	pub fn parse(&self, balance: &str) -> Result<u128> {
		let balance = balance.trim().replace('_', "");
		if !balance.contains('.') && !balance.ends_with(&self.symbol) {
			return balance.parse().map_err(|_| anyhow::anyhow!("balance is not a valid u128"));
		}
		let balance = balance.strip_suffix(&self.symbol).unwrap_or(&balance).trim_end();
		let (unit, balance) = SI_PREFIX
			.iter()
			.find(|(_, p)| balance.ends_with(*p))
			.map_or((0, balance), |(d, p)| (*d, balance.strip_suffix(*p).unwrap()));
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
			(1_200_000, "1.2u ANLG"),
			(1_200_000_000_000_000, "1.2k ANLG"),
		];
		let parse_cases = [("0", 0), ("1_200_000", 1_200_000), ("1.", 1_000_000_000_000)];
		let fmt = BalanceFormatter::new(12, "ANLG");
		for (n, s) in fmt_cases {
			assert_eq!(fmt.format(n), s);
			assert_eq!(fmt.parse(s)?, n);
		}
		for (s, n) in parse_cases {
			assert_eq!(fmt.parse(s)?, n);
		}
		Ok(())
	}
}
