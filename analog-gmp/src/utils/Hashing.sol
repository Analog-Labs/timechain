// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/utils/Hashing.sol)

pragma solidity >=0.8.20;

library Hashing {
    /**
     * @dev Solidity's reserved location for the free memory pointer.
     * Reference: https://docs.soliditylang.org/en/v0.8.28/internals/layout_in_memory.html
     */
    uint256 internal constant ALLOCATED_MEMORY = 0x40;

    /**
     * @dev Solidity's reserved location for the scratch memory.
     * Reference: https://docs.soliditylang.org/en/v0.8.28/internals/layout_in_memory.html
     */
    uint256 internal constant SCRATCH_MEMORY = 0x60;

    /**
     * @dev Hashes a single 256-bit integer without memory allocation, uses the memory between 0x00~0x20.
     */
    function hash(uint256 a) internal pure returns (bytes32 h) {
        assembly ("memory-safe") {
            mstore(0x00, a)
            h := keccak256(0x00, 0x20)
        }
    }

    /**
     * @dev Hashes two 256-bit words without memory allocation, uses the memory between 0x00~0x40.
     */
    function hash(uint256 a, uint256 b) internal pure returns (bytes32 h) {
        assembly ("memory-safe") {
            mstore(0x00, a)
            mstore(0x20, b)
            h := keccak256(0x00, 0x40)
        }
    }

    /**
     * @dev Hashes three 256-bit words without memory allocation, uses the memory between 0x00~0x60.
     *
     * The reserverd memory region `0x40~0x60` is restored to its previous state after execution.
     * See https://docs.soliditylang.org/en/v0.8.28/internals/layout_in_memory.html for more details.
     */
    function hash(uint256 a, uint256 b, uint256 c) internal pure returns (bytes32 h) {
        assembly ("memory-safe") {
            mstore(0x00, a)
            mstore(0x20, b)

            // Backup the free memory pointer
            let freeMemBackup := mload(ALLOCATED_MEMORY)

            mstore(ALLOCATED_MEMORY, c)
            h := keccak256(0x00, 0x60)

            // Restore the free memory pointer
            mstore(ALLOCATED_MEMORY, freeMemBackup)
        }
    }

    /**
     * @dev Hashes four 256-bit words without memory allocation, uses the memory between 0x00~0x80.
     *
     * The reserverd memory regions `0x40` and `0x60` are saved and restored after the hash is computed.
     * See https://docs.soliditylang.org/en/v0.8.28/internals/layout_in_memory.html for more details.
     */
    function hash(bytes32 a, bytes32 b, bytes32 c, bytes32 d) internal pure returns (bytes32) {
        return hash(uint256(a), uint256(b), uint256(c), uint256(d));
    }

    /**
     * @dev Hashes four 256-bit words without memory allocation, uses the memory between 0x00~0x80.
     *
     * The reserverd memory region `0x40~0x80` is restored to its previous state after execution.
     * See https://docs.soliditylang.org/en/v0.8.28/internals/layout_in_memory.html for more details.
     */
    function hash(uint256 a, uint256 b, uint256 c, uint256 d) internal pure returns (bytes32 h) {
        assembly ("memory-safe") {
            mstore(0x00, a)
            mstore(0x20, b)

            // Backup the free memory pointer
            let freeMemBackup := mload(ALLOCATED_MEMORY)
            mstore(ALLOCATED_MEMORY, c)
            {
                // Backup the scratch space 0x60
                let backup := mload(0x60)

                // Compute the hash
                mstore(SCRATCH_MEMORY, d)
                h := keccak256(0x00, 0x80)

                // Restore the scratch space 0x60
                mstore(SCRATCH_MEMORY, backup)
            }
            // Restore the free memory pointer
            mstore(ALLOCATED_MEMORY, freeMemBackup)
        }
    }
}
