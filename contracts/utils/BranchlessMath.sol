// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (contracts/utils/BranchlessMath.sol)

pragma solidity ^0.8.20;

/**
 * @dev Utilities for branchless operations, useful when a constant gas cost is required.
 */
library BranchlessMath {
    /**
     * @dev Returns the smallest of two numbers.
     */
    function min(uint256 x, uint256 y) internal pure returns (uint256) {
        return select(x < y, x, y);
    }

    /**
     * @dev Returns the largest of two numbers.
     */
    function max(uint256 x, uint256 y) internal pure returns (uint256) {
        return select(x > y, x, y);
    }

    /**
     * @dev If `condition` is true returns `a`, otherwise returns `b`.
     */
    function select(bool condition, uint256 a, uint256 b) internal pure returns (uint256) {
        unchecked {
            // branchless select, works because:
            // b ^ (a ^ b) == a
            // b ^ 0 == b
            //
            // This is better than doing `condition ? a : b` because:
            // - Consumes less gas
            // - Constant gas cost regardless the inputs
            // - Reduces the final bytecode size
            return b ^ ((a ^ b) * toUint(condition));
        }
    }

    /**
     * @dev If `condition` is true return `value`, otherwise return zero.
     */
    function selectIf(bool condition, uint256 value) internal pure returns (uint256) {
        unchecked {
            return value * toUint(condition);
        }
    }

    /**
     * @dev Unsigned saturating addition, bounds to UINT256 MAX instead of overflowing.
     * equivalent to:
     * uint256 r = x + y;
     * return r >= x ? r : UINT256_MAX;
     */
    function saturatingAdd(uint256 x, uint256 y) internal pure returns (uint256) {
        unchecked {
            x = x + y;
            y = 0 - toUint(x < y);
            return x | y;
        }
    }

    /**
     * @dev Unsigned saturating subtraction, bounds to zero instead of overflowing.
     * equivalent to: x > y ? x - y : 0
     */
    function saturatingSub(uint256 x, uint256 y) internal pure returns (uint256) {
        unchecked {
            return (x - y) * toUint(x > y);
        }
    }

    /**
     * @dev Unsigned saturating multiplication, bounds to UINT256 MAX instead of overflowing.
     */
    function saturatingMul(uint256 x, uint256 y) internal pure returns (uint256) {
        unchecked {
            x = x * y;
            y = 0 - toUint((x / y) != y);
            return x | y;
        }
    }

    /**
     * @dev Returns the ceiling of the division of two numbers.
     *
     * This differs from standard division with `/` in that it rounds towards infinity instead
     * of rounding towards zero.
     */
    function ceilDiv(uint256 a, uint256 b) internal pure returns (uint256) {
        unchecked {
            // The following calculation ensures accurate ceiling division without overflow.
            // Since a is non-zero, (a - 1) / b will not overflow.
            // The largest possible result occurs when (a - 1) / b is type(uint256).max,
            // but the largest value we can obtain is type(uint256).max - 1, which happens
            // when a = type(uint256).max and b = 1.
            return selectIf(a > 0, ((a - 1) / b + 1));
        }
    }

    /**
     * @dev Cast a boolean (false or true) to a uint256 (0 or 1) with no jump.
     */
    function toUint(bool b) internal pure returns (uint256 u) {
        /// @solidity memory-safe-assembly
        assembly {
            u := iszero(iszero(b))
        }
    }

    /**
     * @dev Cast an address to uint256
     */
    function toUint(address addr) internal pure returns (uint256) {
        return uint256(uint160(addr));
    }
}
