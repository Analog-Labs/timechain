pragma solidity >=0.7.0 <0.9.0;

/**
 * @dev Utilities for branchless operations, useful when a constant gas cost is required.
 */
library BranchlessMath {
    /**
     * @dev Returns the smallest of two numbers.
     */
    function min(uint256 x, uint256 y) internal pure returns (uint256 result) {
        assembly ("memory-safe") {
            // gas efficient branchless min function:
            // min(x,y) = y ^ ((x ^ y) * (x < y))
            // Reference: https://graphics.stanford.edu/~seander/bithacks.html#IntegerMinOrMax
            result := xor(y, mul(xor(x, y), lt(x, y)))
        }
    }

    /**
     * @dev Returns the largest of two numbers.
     */
    function max(uint256 x, uint256 y) internal pure returns (uint256 result) {
        assembly ("memory-safe") {
            // gas efficient branchless max function:
            // max(x,y) = x ^ ((x ^ y) * (x < y))
            // Reference: https://graphics.stanford.edu/~seander/bithacks.html#IntegerMinOrMax
            result := xor(x, mul(xor(x, y), lt(x, y)))
        }
    }

    /**
     * @dev If `cond` is true, use `x`, otherwise use `y`.
     */
    function choice(bool cond, uint256 x, uint256 y) internal pure returns (uint256 result) {
        assembly ("memory-safe") {
            // gas efficient branchless choice function:
            // choice(c,x,y) = x ^ ((x ^ y) * (c == 0))
            result := xor(x, mul(xor(x, y), iszero(cond)))
        }
    }

    /**
     * @dev Unsigned saturating addition, bounds to UINT256 MAX instead of overflowing.
     * equivalent to:
     * uint256 r = x + y;
     * return r >= x ? r : UINT256_MAX;
     */
    function saturatingAdd(uint256 x, uint256 y) internal pure returns (uint256 result) {
        assembly ("memory-safe") {
            // add(x,y) = (x + y) | -(y > (x + y))
            x := add(x, y)
            y := sub(0, gt(y, x))
            result := or(x, y)
        }
    }

    /**
     * @dev Unsigned saturating subtraction, bounds to zero instead of overflowing.
     * equivalent to: x > y ? x - y : 0
     */
    function saturatingSub(uint256 x, uint256 y) internal pure returns (uint256 result) {
        assembly ("memory-safe") {
            // sub(x,y) = (x - y) * (x > y)
            result := mul(sub(x, y), gt(x, y))
        }
    }

    /**
     * @dev Convert a bool into uint256, solidity doesn't allow this conversion.
     * equivalent to: x > y ? x - y : 0
     */
    function boolToUint(bool b) internal pure returns (uint256 value) {
        assembly ("memory-safe") {
            value := iszero(iszero(b))
        }
    }
}
