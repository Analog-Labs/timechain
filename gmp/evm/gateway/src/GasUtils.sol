// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/utils/GasUtils.sol)

pragma solidity >=0.8.20;

import {PrimitiveUtils} from "./Primitives.sol";

/**
 * @dev Utilities for compute the GMP gas price, gas cost and gas needed.
 */
library GasUtils {
    using PrimitiveUtils for uint256;

    function calldataSize(uint16 messageSize) internal pure returns (uint256) {
        return uint256(messageSize).align32() + 676; // selector + Signature + Batch
    }

    /**
     * @dev Compute the amount of gas used by the `GatewayProxy`.
     * @param calldataLen The length of the calldata in bytes
     */
    function proxyOverheadGas(uint256 calldataLen) internal pure returns (uint256) {
        unchecked {
            // Base cost: OPCODES + COLD SLOAD + COLD DELEGATECALL + RETURNDATACOPY
            uint256 gas = 31 + 2100 + 2600 + 32 + 66;

            // CALLDATACOPY
            gas += calldataLen.toWordCount() * 3;

            // RETURNDATACOPY
            // gas += returnLen.toWordCount() * 3;

            // MEMORY EXPANSION (minimal 3 due mstore(0x40, 0x80))
            gas += memoryExpansionGas(calldataLen.toWordCount());
            return gas;
        }
    }

    /**
     * @dev Compute the gas cost of memory expansion.
     * @param words number of words, where a word is 32 bytes
     */
    function memoryExpansionGas(uint256 words) internal pure returns (uint256) {
        unchecked {
            return ((words * words) >> 9) + words * 3;
        }
    }

    /**
     * @dev Compute the transaction base cost.
     */
    function txBaseGas() internal pure returns (uint256) {
        unchecked {
            uint256 nonZeros = countNonZerosCalldata(msg.data);
            uint256 zeros = msg.data.length - nonZeros;
            return 21000 + (nonZeros * 16) + (zeros * 4);
        }
    }

    /**
     * @dev Count the number of non-zero bytes from calldata.
     * gas cost = 224 + (words * 106) + (((words + 254) / 255) * 214)
     */
    function countNonZerosCalldata(bytes calldata data) internal pure returns (uint256 nonZeros) {
        assembly ("memory-safe") {
            nonZeros := 0
            for {
                let ptr := data.offset
                let end := add(ptr, data.length)
            } lt(ptr, end) {} {
                // calculate min(ptr + data.length, ptr + 8160)
                let range := add(ptr, 8160)
                range := xor(end, mul(xor(range, end), lt(range, end)))

                // Normalize and count non-zero bytes in parallel
                let v := 0
                for {} lt(ptr, range) { ptr := add(ptr, 32) } {
                    let r := calldataload(ptr)
                    r := or(r, shr(4, r))
                    r := or(r, shr(2, r))
                    r := or(r, shr(1, r))
                    r := and(r, 0x0101010101010101010101010101010101010101010101010101010101010101)
                    v := add(v, r)
                }

                // Sum bytes in parallel
                {
                    let l := and(v, 0x00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff)
                    v := shr(8, xor(v, l))
                    v := add(v, l)
                }
                v := add(v, shr(128, v))
                v := add(v, shr(64, v))
                v := add(v, shr(32, v))
                v := add(v, shr(16, v))
                v := and(v, 0xffff)
                nonZeros := add(nonZeros, v)
            }
        }
    }
}
