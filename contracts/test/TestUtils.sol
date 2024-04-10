// SPDX-License-Identifier: GPL-3.0

pragma solidity >=0.7.0 <0.9.0;

import "forge-std/Test.sol";

/**
 * @dev Utilities for testing purposes
 */
library TestUtils {
    // Cheat code address, 0x7109709ECfa91a80626fF3989D68f67F5b1DD12D.
    address internal constant VM_ADDRESS = address(uint160(uint256(keccak256("hevm cheat code"))));

    Vm internal constant vm = Vm(VM_ADDRESS);

    /**
     * @dev Deploys a contract with the given bytecode
     */
    function deployContract(bytes memory bytecode) internal returns (address addr) {
        require(bytecode.length > 0, "Error: deploy code is empty");
        assembly ("memory-safe") {
            let ptr := add(bytecode, 32)
            let size := mload(bytecode)
            addr := create(0, ptr, size)
        }
        require(addr != address(0), "Error: failed to deploy contract");
    }

    /**
     * @dev  Delegate call to another contract bytecode
     *  This execute the code of another contract in the context of the current contract
     */
    function delegateCall(address contractAddr, bytes memory data)
        internal
        returns (bool success, bytes memory output)
    {
        require(contractAddr.code.length > 0, "Error: provided address is not a contract");
        assembly ("memory-safe") {
            success :=
                delegatecall(
                    gas(), // call gas limit
                    contractAddr, // dest address
                    add(32, data), // input memory pointer
                    mload(data), // input size
                    0, // output memory pointer
                    0 // output size
                )

            // Alloc memory for the output
            output := mload(0x40)
            let ptr := add(output, 32)
            let size := returndatasize()
            mstore(0x40, add(ptr, size)) // Increment free memory pointer

            // Store return data size
            mstore(output, size)

            // Copy delegatecall output to memory
            returndatacopy(ptr, 0, size)
        }
    }

    /**
     * @dev Count non-zero bytes in a 256bit word in parallel
     * Reference: https://graphics.stanford.edu/~seander/bithacks.html#CountBitsSetParallel
     */
    function countNonZeros(bytes memory data) internal pure returns (uint256 nonZeros) {
        assembly ("memory-safe") {
            nonZeros := 0
            for {
                let len := mload(data)
                let ptr := add(data, 32)
                let end := add(ptr, len)
            } lt(ptr, end) { ptr := add(ptr, 32) } {
                let v := mload(ptr)
                v := or(v, shr(4, v))
                v := or(v, shr(2, v))
                v := or(v, shr(1, v))
                v := and(v, 0x0101010101010101010101010101010101010101010101010101010101010101)
                v := add(v, shr(128, v))
                v := add(v, shr(64, v))
                v := add(v, shr(32, v))
                v := add(v, shr(16, v))
                v := add(v, shr(8, v))
                v := and(v, 0xff)
                nonZeros := add(nonZeros, v)
            }
        }
    }

    /**
     * @dev Calculate the tx base cost.
     * formula: 21000 + zeros * 4 + nonZeros * 16
     * Reference: https://eips.ethereum.org/EIPS/eip-2028
     */
    function calculateBaseCost(bytes memory txData) internal pure returns (uint256 baseCost) {
        uint256 nonZeros = countNonZeros(txData);
        uint256 zeros = txData.length - nonZeros;
        baseCost = 21_000 + (nonZeros * 16) + (zeros * 4);
    }

    // Get calldata
    function getCalldata() internal pure returns (bytes memory out) {
        assembly ("memory-safe") {
            out := mload(0x40)
            let ptr := add(out, 32)
            let size := calldatasize()
            mstore(0x40, add(ptr, size)) // Increment free memory pointer
            mstore(out, size)
            calldatacopy(ptr, 0, size)
        }
    }

    /**
     * @dev Generate a new account account from the calldata
     * This will generate a unique deterministic address for each test case
     */
    function createTestAccount(uint256 initialBalance) internal returns (address account) {
        // Generate a new account address from the calldata
        // This will generate a unique deterministic address for each test case
        account = address(uint160(uint256(keccak256(getCalldata()))));
        vm.deal(account, initialBalance);
    }

    /**
     * @dev Generate a new account account from the calldata
     */
    function createTestAccount() internal returns (address account) {
        // Create an account with 100 ether
        account = createTestAccount(100 ether);
    }

    /**
     * @dev Convert an address to GMP bytes32 identifier
     */
    function source(address account, bool isContract) internal pure returns (bytes32) {
        uint256 contractFlag = isContract ? 1 << 160 : 0;
        return bytes32(contractFlag | uint256(uint160(account)));
    }

    /**
     * @dev Convert an address to GMP bytes32 identifier
     */
    function source(address account) internal view returns (bytes32) {
        return source(account, account.code.length > 0);
    }
}
