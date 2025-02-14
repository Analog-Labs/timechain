// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Float9x56.t.sol)

pragma solidity >=0.8.0;

import {Test, console} from "forge-std/Test.sol";
import {BranchlessMath, Rounding} from "../src/utils/BranchlessMath.sol";
import {UFloat9x56, UFloatMath} from "../src/utils/Float9x56.sol";

contract UFloatMathMock {
    function encode(uint256 mantissa, int256 exponent) external pure returns (UFloat9x56) {
        return UFloatMath.encode(mantissa, exponent);
    }
}

contract UFloatMathTest is Test {
    using UFloatMath for UFloat9x56;
    using BranchlessMath for uint256;

    UFloatMathMock mock;

    // Fuzz test fixtures for mantissa, see:
    // - https://book.getfoundry.sh/forge/fuzz-testing#fuzz-test-fixtures
    uint56[] public fixtureMantissa = [0, 1, uint56(UFloatMath.MANTISSA_MIN), uint56(UFloatMath.MANTISSA_MAX)];

    constructor() {
        mock = new UFloatMathMock();
    }

    /**
     * @dev multiply an UFloat9x56 by an uint256 in constant gas.
     */
    function test_fuzzMul(uint256 x) external pure {
        // Any value multiplied by 0 should be 0
        uint256 result = UFloatMath.ZERO.mul(x);
        assertEq(result, 0);

        // Any value times 1 should be the same value
        result = UFloatMath.ONE.mul(x);
        assertEq(result, x);

        // Any value times 2 should be the double
        result = UFloatMath.fromUint(2).mul(x);
        unchecked {
            assertEq(result, x << 1);
        }
    }

    function test_mul() external pure {
        unchecked {
            // Any value multiplied by 0 should be 0
            UFloat9x56 value = UFloatMath.ZERO;
            assertEq(value.mul(0), 0);
            assertEq(value.mul(1), 0);
            assertEq(value.mul(type(uint256).max), 0);

            // Any value multiplied 1 should be the same value
            value = UFloatMath.ONE;
            assertEq(value.mul(0), 0);
            assertEq(value.mul(1), 1);
            assertEq(value.mul(type(uint256).max), type(uint256).max);

            // Any value times 2 should be the double
            value = UFloatMath.fromUint(2);
            assertEq(value.mul(0), 0);
            assertEq(value.mul(1), 2);
            assertEq(value.mul(type(uint256).max), type(uint256).max * 2);

            // Test fractions
            value = UFloatMath.fromRational(1, 2, Rounding.Floor);
            assertEq(value.mul(0), 0);
            assertEq(value.mul(1), 0);
            assertEq(value.mul(2), 1);
            assertEq(value.mul(type(uint256).max), type(uint256).max / 2);
        }
    }

    function test_eq() external pure {
        unchecked {
            assertTrue(UFloatMath.ZERO.eq(0));
            assertFalse(UFloatMath.ZERO.eq(1));
            assertFalse(UFloatMath.ZERO.eq(type(uint256).max));

            assertFalse(UFloatMath.ONE.eq(0));
            assertTrue(UFloatMath.ONE.eq(1));
            assertFalse(UFloatMath.ONE.eq(type(uint256).max));

            UFloat9x56 value = UFloatMath.fromUint(type(uint256).max);
            assertFalse(value.eq(0));
            assertFalse(value.eq(1));
            assertFalse(value.eq(type(uint256).max / 2));
            assertTrue(value.eq(type(uint256).max));

            value = UFloatMath.fromRational(123456789, 1111);
            assertFalse(value.eq(0));
            assertFalse(value.eq(1));
            assertFalse(value.eq(type(uint256).max));
            assertTrue(value.eq(uint256(123456789) / uint256(1111)));
        }
    }

    /**
     * @dev Saturating multiplication, bounds to `2 ** 256 - 1` instead of overflowing.
     */
    function test_saturatingMul() external pure {
        unchecked {
            // Any value multiplied by 0 should be 0
            UFloat9x56 value = UFloatMath.ZERO;
            assertEq(value.saturatingMul(0), 0);
            assertEq(value.saturatingMul(1), 0);
            assertEq(value.saturatingMul(type(uint256).max), 0);

            // Any value multiplied 1 should be the same value
            value = UFloatMath.ONE;
            assertEq(value.saturatingMul(0), 0);
            assertEq(value.saturatingMul(1), 1);
            assertEq(value.saturatingMul(type(uint256).max), type(uint256).max);

            // Any value times 2 should be the double or bounded to 2 ** 256 - 1
            value = UFloatMath.fromUint(2);
            assertEq(value.saturatingMul(0), 0);
            assertEq(value.saturatingMul(1), 2);
            assertEq(value.saturatingMul(type(uint256).max - 1), type(uint256).max);
            assertEq(value.saturatingMul(type(uint256).max), type(uint256).max);

            // Test fractions
            value = UFloatMath.fromRational(1, 2, Rounding.Floor);
            assertEq(value.saturatingMul(0), 0);
            assertEq(value.saturatingMul(1), 0);
            assertEq(value.saturatingMul(2), 1);
            assertEq(value.saturatingMul(type(uint256).max), type(uint256).max / 2);
        }
    }

    /**
     * @dev Returns the mantissa and base 2 exponent as integers, respectively.
     * The original number can be recovered by `mantissa * 2 ** exponent`.
     * Returns (0, -311) if the value is zero.
     */
    function test_decode() external pure {
        (uint256 mantissa, int256 exponent) = UFloatMath.ZERO.decode();
        assertEq(mantissa, 0);
        assertEq(exponent, -311);

        (mantissa, exponent) = UFloatMath.ONE.decode();
        assertEq(mantissa, UFloatMath.MANTISSA_MIN);
        assertEq(exponent, -55);

        (mantissa, exponent) = UFloatMath.MAX.decode();
        assertEq(mantissa, 0xffffffffffffff);
        assertEq(exponent, 200);

        (mantissa, exponent) = UFloatMath.fromRational(1, 3).decode();
        assertEq(mantissa, 0xaaaaaaaaaaaaab);
        assertEq(exponent, -57);

        (mantissa, exponent) = UFloatMath.fromRational(123456789123456789, 1000000000).decode();
        assertEq(mantissa, 0xeb79a2a3f35ba7);
        assertEq(exponent, -29);
    }

    /**
     * @dev Test encoding `mantissa` and `exponent` into `UFloat9x56`
     */
    function test_encode() external {
        // Encode zero
        assertEq(UFloatMath.encode(0, 0), UFloatMath.ZERO);
        assertEq(UFloatMath.encode(0, -311), UFloatMath.ZERO);
        assertEq(UFloatMath.encode(0, -310), UFloatMath.ZERO);
        assertEq(UFloatMath.encode(0, 200), UFloatMath.ZERO);

        // Normal numbers
        assertEq(UFloatMath.encode(0x80000000000000, -55), UFloatMath.ONE);
        assertEq(UFloatMath.encode(UFloatMath.MANTISSA_MAX, 200), UFloatMath.MAX);
        assertEq(UFloatMath.encode(0xaaaaaaaaaaaaab, -57), UFloatMath.fromRational(1, 3));
        assertEq(UFloatMath.encode(0xeb79a2a3f35ba7, -29), UFloatMath.fromRational(123456789123456789, 1000000000));
        assertEq(UFloatMath.encode(UFloatMath.MANTISSA_MIN, -310), UFloat9x56.wrap(0x0080000000000000));

        // Subnormal numbers
        assertEq(UFloatMath.encode(0x00000000000000, -311), UFloat9x56.wrap(0x0000000000000000));
        assertEq(UFloatMath.encode(0x00000000000001, -310), UFloat9x56.wrap(0x0000000000000001));
        assertEq(UFloatMath.encode(0x7fffffffffffff, -310), UFloat9x56.wrap(0x007fffffffffffff));

        // Revert if the exponent is out of bounds
        vm.expectRevert("UFloat9x56: exponent out of bounds");
        mock.encode(0, -312);
        vm.expectRevert("UFloat9x56: exponent out of bounds");
        mock.encode(0, 201);

        // Revert if the mantissa is invalid
        vm.expectRevert("UFloat9x56: invalid mantissa");
        mock.encode(1, 0);
        vm.expectRevert("UFloat9x56: invalid mantissa");
        mock.encode(UFloatMath.MANTISSA_MAX + 1, 200);
        vm.expectRevert("UFloat9x56: invalid mantissa");
        mock.encode(UFloatMath.MANTISSA_MIN - 1, -309);
    }

    /**
     * @dev Fuzz test converting between `uint256` and `UFloat9x56`.
     * The conversion must be exact given x is 56 bit long.
     */
    function test_fuzzConvertUint(uint56 mantissa, uint8 exponent) external pure {
        unchecked {
            uint256 value = uint256(mantissa) << uint256(exponent);
            UFloat9x56 float = UFloatMath.fromUint(value);
            assertEq(float.truncate(), value);
        }
    }

    /**
     * @dev Fuzz test `UFloatMath.mul` and `UFloatMath.saturatingMul`.
     */
    function test_fuzzMultiplication(uint56 mantissa, uint8 exponent, uint256 multiplier) external pure {
        unchecked {
            uint256 value = uint256(mantissa) << uint256(exponent);
            UFloat9x56 float = UFloatMath.fromUint(value);
            assertEq(float.mul(multiplier), value * multiplier);
            assertEq(float.saturatingMul(multiplier), value.saturatingMul(multiplier));
        }
    }

    /**
     * @dev Compare two `UFloat`.
     */
    function assertEq(UFloat9x56 left, UFloat9x56 right) internal pure {
        assertEq(UFloat9x56.unwrap(left), UFloat9x56.unwrap(right));
    }

    function assertEq(UFloat9x56 left, UFloat9x56 right, string memory message) internal pure {
        assertEq(UFloat9x56.unwrap(left), UFloat9x56.unwrap(right), message);
    }

    /**
     * @dev Computes `num * 2**exp / den` with the given rounding direction.
     */
    function muldiv(uint256 num, int256 exp, uint256 den, Rounding rounding) internal pure returns (uint256) {
        uint256 numzeros = num.leadingZeros();
        uint256 denzeros = den.leadingZeros();
        assertLe(exp, int256(denzeros));
        assertGe(exp, -int256(numzeros + 255));

        if (exp < -255) {
            num <<= uint256(-exp) - 255;
            exp = 255;
        } else if (exp <= 0) {
            exp = -exp;
        } else {
            den <<= uint256(exp);
            exp = 0;
        }

        return BranchlessMath.mulDiv(num, 1 << uint256(exp), den, rounding);
    }

    /**
     * @dev When `numerator` and `denominator` are exact, the conversion must be exact.
     */
    function checkfromRationalRouding(uint256 num, uint256 den, UFloat9x56 expected) internal pure {
        (uint256 mantissa, int256 exponent) = expected.decode();
        assertGt(mantissa + BranchlessMath.toUint(expected.eq(UFloatMath.ZERO)), 0, "mantissa is zero");
        uint256 value = muldiv(num, exponent, den, Rounding.Floor);
        assertEq(mantissa, value, "Rounding.Floor failed");

        bool roundUp = muldiv(num, exponent, den, Rounding.Nearest) > value && expected.lt(UFloatMath.MAX);
        expected = UFloat9x56.wrap(uint64(UFloat9x56.unwrap(expected) + BranchlessMath.toUint(roundUp)));
        (mantissa, exponent) = expected.decode();
        value = muldiv(num, exponent, den, Rounding.Nearest);
        assertEq(mantissa, value, "Rounding.Nearest failed");

        roundUp = muldiv(num, exponent, den, Rounding.Ceil) > value && expected.lt(UFloatMath.MAX);
        expected = UFloat9x56.wrap(uint64(UFloat9x56.unwrap(expected) + BranchlessMath.toUint(roundUp)));
        (mantissa, exponent) = expected.decode();
        value = muldiv(num, exponent, den, Rounding.Ceil);
        assertEq(mantissa, value, "Rounding.Ceil failed");
    }

    /**
     * @dev Converts numerator / denominator to `UFloat9x56`, following the selected rounding direction.
     */
    function test_fromRational() external pure {
        // Normal numbers
        checkfromRationalRouding(0, 1, UFloatMath.ZERO);
        checkfromRationalRouding(0, 2, UFloatMath.ZERO);
        checkfromRationalRouding(0, type(uint256).max, UFloatMath.ZERO);
        checkfromRationalRouding(1, 1, UFloatMath.ONE);
        checkfromRationalRouding(2, 2, UFloatMath.ONE);
        checkfromRationalRouding(UFloatMath.MANTISSA_MAX, UFloatMath.MANTISSA_MAX, UFloatMath.ONE);
        checkfromRationalRouding(type(uint256).max, type(uint256).max, UFloatMath.ONE);
        checkfromRationalRouding(0x2ffffffffffffff, 3, UFloat9x56.wrap(0x9bffffffffffffff));

        // Test rounding
        assertEq(UFloatMath.fromRational(0x2ffffffffffffff, 3, Rounding.Floor), UFloat9x56.wrap(0x9bffffffffffffff));
        assertEq(UFloatMath.fromRational(0x2ffffffffffffff, 3, Rounding.Nearest), UFloat9x56.wrap(0x9c00000000000000));
        assertEq(UFloatMath.fromRational(0x2ffffffffffffff, 3, Rounding.Ceil), UFloat9x56.wrap(0x9c00000000000000));
    }

    /**
     * @dev Converts numerator / denominator to `UFloat9x56`, following the selected rounding direction.
     */
    function test_fuzzFromRational(uint248 numerator, uint248 denominator) external pure {
        vm.assume(denominator > 0 && numerator > 0);
        uint256 numbits = BranchlessMath.log2(numerator);
        uint256 denbits = BranchlessMath.log2(denominator);
        vm.assume(numbits.absDiff(denbits) <= 200);
        unchecked {
            UFloat9x56 float = UFloatMath.fromRational(numerator, denominator, Rounding.Floor);
            checkfromRationalRouding(numerator, denominator, float);
            uint256 integer = uint256(numerator) / uint256(denominator);
            assertTrue(float.eq(integer), "float is not equal to integer");

            // Find a multiplier such that (multipler / numerator) * numerator > 0
            uint256 multiplier = denbits.saturatingSub(numbits) + UFloatMath.MANTISSA_DIGITS - 1;
            multiplier = multiplier.min(255 - numbits);
            multiplier = 2 ** multiplier;

            // Calculate numerator * multipler / numerator
            integer = BranchlessMath.mulDiv(numerator, multiplier, denominator, Rounding.Floor);
            {
                // Keep only the `MANTISSA_DIGITS` most significant bits.
                uint256 shift = integer.log2().saturatingSub(UFloatMath.MANTISSA_DIGITS - 1);
                uint256 mask = type(uint256).max << shift;
                integer &= mask;
            }
            assertGt(integer, 0, "integer is zero");
            assertEq(float.mul(multiplier), integer);
        }
    }

    /**
     * @dev Convert `UFloat9x56` to a rational number, returns the numerator and denominator, respectively.
     * Obs: Values above 2**-256 are represented precisely, values below are approximated or round down to zero.
     */
    function test_toRational(uint56 mantissa, uint16 exponent) external pure {
        // Make sure the exponent is within 9 bit bounds.
        exponent %= 2 ** 9;

        // Doesn't allow exponent == 0, once subnormal numbers cannot be converted to rational precisely.
        vm.assume(exponent > 0);
        unchecked {
            UFloat9x56 float =
                UFloat9x56.wrap(uint64(mantissa) | uint64(exponent) << uint64(UFloatMath.MANTISSA_DIGITS - 1));
            (uint256 numerator, uint256 denominator) = float.toRational();
            uint256 numbits = numerator.log2();
            uint256 denbits = denominator.log2();

            uint256 integer = uint256(numerator) / uint256(denominator);
            assertTrue(float.eq(integer), "float is not equal to integer");

            // Find a multiplier such that (multipler / numerator) * numerator > 0
            uint256 multiplier = denbits.saturatingSub(numbits) + UFloatMath.MANTISSA_DIGITS - 1;
            multiplier = multiplier.min(255 - numbits);
            multiplier = 2 ** multiplier;

            // Calculate numerator * multipler / numerator
            integer = BranchlessMath.mulDiv(numerator, multiplier, denominator, Rounding.Floor);
            {
                // Keep only the `MANTISSA_DIGITS` most significant bits.
                uint256 shift = integer.log2().saturatingSub(UFloatMath.MANTISSA_DIGITS - 1);
                uint256 mask = type(uint256).max << shift;
                integer &= mask;
            }
            assertGt(integer, 0, "integer is zero");
            assertEq(float.mul(multiplier), integer);
        }
    }
}
