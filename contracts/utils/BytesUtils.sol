// SPDX-License-Identifier: GPL-3.0

pragma solidity >=0.7.0 <0.9.0;

/**
 * @dev Utilities for manipulate bytes.
 */
library BytesUtils {
    /**
     * @dev Inject code in the bytecode
     */
    function inject(bytes32[] memory table, bytes memory bytecode) internal pure {
        assembly ("memory-safe") {
            let size := mload(bytecode)
            let offset := add(bytecode, 32)
            let end := add(bytecode, size)
            let codes := mload(table)

            for {} and(gt(codes, 0), lt(offset, end)) {} {
                // Efficient Algorithm to find 32 consecutive repeated bytes in a byte sequence
                for { let chunk := 1 } gt(chunk, 0) { offset := add(offset, chunk) } {
                    // Transform all `0x7F` bytes into `0xFF`
                    // 0x80 ^ 0x7F == 0xFF
                    // Also transform all other bytes in something different than `0xFF`
                    chunk := xor(mload(offset), 0x8080808080808080808080808080808080808080808080808080808080808080)

                    // Find the right most unset bit, which is equivalent to find the
                    // right most byte different than `0x7F`.
                    // ex: (0x12345678FFFFFF + 1) & (~0x12345678FFFFFF) == 0x00000001000000
                    chunk := and(add(chunk, 1), not(chunk))

                    // Round down to the closest power of 2 multiple of 256
                    chunk := div(chunk, mod(chunk, 0xff))

                    // Find the number of leading bytes different than `0x7F`.
                    // Rationale:
                    // Multiplying a number by a power of 2 is the same as shifting the bits to the left
                    // ex: 1337 * (2 ** 16) == 1337 << 16
                    // Once the chunk is a multiple of 256 it always shift entire bytes, we use this to
                    // select a specific byte in a byte sequence.
                    chunk := byte(0, mul(0x201f1e1d1c1b1a191817161514131211100f0e0d0c0b0a090807060504030201, chunk))

                    // Stop the loop if we go out of bounds
                    // obs: can remove this check if you are 100% sure the constant exists
                    chunk := mul(chunk, lt(offset, end))
                }
                offset := add(offset, 1)
                let tag := sub(byte(31, mload(offset)), 1)
                let pos := add(add(table, 32), shl(5, tag))
                let code := mload(pos)
                if or(gt(tag, size), iszero(byte(0, code))) { revert(0, 0) }
                mstore(pos, 0)
                mstore(offset, 0x5B)
                mstore(sub(offset, 1), code)
                offset := add(offset, 32)
                codes := sub(codes, 1)
            }

            if gt(codes, 0) { revert(0, 0) }
        }
    }
}
