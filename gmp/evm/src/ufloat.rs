// Ref: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=8d572811c499b6a45f6b7ba3c98f3f1c
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::{One, Zero};

#[derive(Debug, PartialEq, Eq)]
pub enum ConversionError {
	/// The denominator is zero
	DivisionByZero,
}

/// Basic float conversion and helper methods, supports custom 64bit floats.
/// OBS: Doesn't support NaNs, Subnormal numbers, infinities and negative values.
pub trait Float {
	/// Number of bits for represent the exponent
	const EXPONENT_DIGITS: u32;
	/// Number of bits used to represent the mantissa
	const MANTISSA_DIGITS: u32;
	/// The zero exponent offset
	const EXPONENT_OFFSET: u32;
	/// Wether the float supports `not a number` or not.
	const HAS_NAN: bool;

	/// Convert a rational number to float
	/// # Errors
	/// Returns an error if the value is too big or too tiny to be represented.
	#[allow(
		clippy::cast_possible_wrap,
		clippy::cast_possible_truncation,
		clippy::unwrap_used,
		clippy::missing_errors_doc
	)]
	fn rational_to_float<N, D>(
		(num, den): (N, D),
		rounding: Rounding,
	) -> Result<u64, ConversionError>
	where
		N: Into<BigUint>,
		D: Into<BigUint>,
	{
		let num = num.into();
		let den = den.into();

		if den.is_zero() {
			return Err(ConversionError::DivisionByZero);
		}

		// Trivial case for zero
		if num.is_zero() {
			return Ok(0);
		}

		// Compute the number of bits required to represent the numerator and denominator.
		let num_bits = num.bits() as i32;
		let den_bits = den.bits() as i32;

		// Try an exponent that will make the mantissa fit in the available bits.
		let mut exponent = (Self::MANTISSA_DIGITS as i32) - (num_bits - den_bits);

		// Extract the mantissa by computing: numerator * 2^exponent / denominator.
		let mut mantissa = mul_div(exponent, num.clone(), den.clone(), rounding);

		// If the mantissa is too large, adjust the exponent.
		let mantissa_max = (BigUint::one() << Self::MANTISSA_DIGITS) - 1u32;
		if mantissa > mantissa_max {
			exponent -= 1;
			mantissa = mul_div(exponent, num, den, rounding);
		}

		// Normalize mantissa to be a value between 1~0
		exponent = -exponent;
		exponent += (Self::MANTISSA_DIGITS - 1) as i32;
		exponent += Self::EXPONENT_OFFSET as i32;

		// Verify if the float is subnormal number (msb is not set)
		if exponent < 0 {
			mantissa =
				mul_div(exponent, mantissa, BigUint::one() << exponent.unsigned_abs(), rounding);
			exponent = 0;
		}
		let mut exponent = u64::from(exponent.unsigned_abs());

		// If the the exponent is too big to be represented, bound it to the maximum value.
		let max_exponent = (1 << Self::EXPONENT_DIGITS) - 1;
		if exponent > max_exponent {
			exponent = max_exponent;
			if Self::HAS_NAN {
				// Set the mantissa to zero, to represent infinity instead of NaN
				mantissa = BigUint::zero();
			} else {
				// Set mantissa to the maximum value
				mantissa.clone_from(&mantissa_max);
			}
		}

		// Remove most significant bit from mantissa (it is always set)
		mantissa &= mantissa_max >> 1;

		// Shift the exponent to the right position
		exponent <<= Self::MANTISSA_DIGITS - 1;

		// Convert mantissa and exponent to float
		Ok(mantissa.to_u64_digits().first().unwrap_or(&0u64) | exponent)
	}
}

// Ref: https://github.com/Analog-Labs/analog-gmp/blob/5653d1dc756e93072e060ab0bbc0b8822931e2c0/src/utils/Float9x56.sol#L13-L117
pub enum UFloat9x56 {}
impl Float for UFloat9x56 {
	/// Number of bits for represent the exponent
	const EXPONENT_DIGITS: u32 = 9;
	/// Number of bits for represent the mantissa (msb is always set)
	const MANTISSA_DIGITS: u32 = 56;
	/// The zero exponent in raw format.
	/// which is: 0x8000000000000000 >> 55
	const EXPONENT_OFFSET: u32 = 256;
	/// `UFloat9x56` doesn't support NaN or infinity.
	const HAS_NAN: bool = false;
}

/// Rounding mode used when converting rational to float.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
	/// Rounds to the nearest value, if the number falls midway,
	/// it is rounded to the nearest value with an even least significant digit.
	NearestTiesEven,
	/// Rounds towards infinite
	Up,
}

/// Computes: numerator * 2^exponent / denominator.
#[allow(clippy::cast_possible_truncation)]
#[inline]
fn mul_div<N, D>(exponent: i32, numerator: N, denominator: D, rounding: Rounding) -> BigUint
where
	N: Into<BigUint>,
	D: Into<BigUint>,
{
	let mut numerator = numerator.into();
	let denominator = denominator.into();

	// Multiply the numerator by 2^exponent
	if exponent >= 0 {
		numerator <<= exponent.unsigned_abs();
	} else {
		numerator >>= exponent.unsigned_abs();
	};

	// Round the result
	let remainder = match rounding {
		Rounding::NearestTiesEven => {
			let mut rem = &denominator >> 1u32;
			if !rem.is_zero() && denominator.is_even() {
				// Round towards even
				let is_odd =
					mul_div(0, numerator.clone(), denominator.clone(), Rounding::Up).is_odd();
				rem -= u32::from(is_odd);
			}
			rem
		},
		Rounding::Up => &denominator - BigUint::one(),
	};

	(numerator + remainder) / denominator
}
