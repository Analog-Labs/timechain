// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Random.sol)

pragma solidity >=0.8.0;

/**
 * @dev Utilities for generating pseudo-random values
 */
library Random {
    function _next() private pure returns (uint256 rand) {
        assembly ("memory-safe") {
            rand := keccak256(0x00, 0x60)
            let ptr := mload(0x40)
            mstore(0x00, xor(rand, calldataload(0)))
            mstore(0x20, xor(rand, calldatasize()))
            mstore(0x40, xor(rand, mload(ptr)))
            rand := keccak256(0x00, 0x60)
            mstore(0x00, rand)
            mstore(0x20, rand)
            mstore(0x40, ptr)
            mstore(ptr, rand)
        }
    }

    function nextUint() internal pure returns (uint256 rand) {
        rand = _next();
    }

    function nextInt() internal pure returns (int256 rand) {
        rand = int256(_next());
    }
}
