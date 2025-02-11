// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/utils/GasUtils.sol)

pragma solidity >=0.8.20;

import {UFloat9x56, UFloatMath} from "./Float9x56.sol";
import {BranchlessMath} from "./BranchlessMath.sol";

/**
 * @dev Utilities for compute the GMP gas price, gas cost and gas needed.
 */
library GasUtils {
    using BranchlessMath for uint256;

    /**
     * @dev How much gas is used until the first `gasleft()` instruction is executed in the `Gateway.batchExecute` method.
     *
     * HOW TO UPDATE THIS VALUE:
     * 1. Run `forge test --match-test=test_gasMeter --fuzz-runs=1 --debug`
     * 2. Move the cursor until you enter the `src/Gateway.sol` file.
     * 3. Execute the opcodes until you reach the first `GAS` opcode.
     * 4. Execute the GAS opcode then copy the `Gas used in call` value to the constant below.
     *
     * Obs: To guarantee the overhead is constant regardless the input size, always use `calldata` instead of `memory`
     * for external functions.
     */
    uint256 internal constant BATCH_SELECTOR_OVERHEAD = 465;

    /**
     * @dev How much gas is used until the first `gasleft()` instruction is executed.
     *
     * HOW TO UPDATE THIS VALUE:
     * 1. Run `forge test --match-test=test_submitMessageMeter --fuzz-runs=1 --debug`
     * 2. Move the cursor until you enter the `src/Gateway.sol` file.
     * 3. Execute the opcodes until you reach the first `GAS` opcode.
     * 4. Execute the GAS opcode then copy the `Gas used in call` value to the constant below.
     *
     * Obs: To guarantee the overhead is constant regardless the input size, always use `calldata` instead of `memory`
     * for external functions.
     */
    uint256 internal constant EXECUTION_SELECTOR_OVERHEAD = 474;

    /**
     * @dev Base cost of the `IExecutor.execute` method.
     */
    uint256 internal constant EXECUTION_BASE_COST = EXECUTION_SELECTOR_OVERHEAD + 46960;

    /**
     * @dev Base cost of the `IGateway.submitMessage` method.
     */
    uint256 internal constant SUBMIT_BASE_COST = 24138;

    /**
     * @dev Extra gas cost that any account `Contract or EOA` must pay when calling `IGateway.submitMessage` method.
     * This cost is necessary for initialize the account's `nonce` storage slot.
     */
    uint256 internal constant FIRST_MESSAGE_EXTRA_COST = 17100;

    /**
     * @dev Solidity's reserved location for the free memory pointer.
     * Reference: https://docs.soliditylang.org/en/v0.8.28/internals/layout_in_memory.html
     */
    uint256 internal constant ALLOCATED_MEMORY = 0x40;

    /**
     * @dev Read the current allocated size (a.k.a free memory pointer).
     */
    function readAllocatedMemory() internal pure returns (uint256 pointer) {
        assembly ("memory-safe") {
            pointer := mload(ALLOCATED_MEMORY)
        }
    }

    /**
     * @dev Replace the current allocated size by the `newPointer`, and returns the old value stored.
     * CAUTION: Only use this method if you know what you are doing. Make sure you don't overwrite any
     * memory location that is still in use by the current call context.
     */
    function unsafeReplaceAllocatedMemory(uint256 newPointer) internal pure returns (uint256 oldPointer) {
        assembly ("memory-safe") {
            oldPointer := mload(ALLOCATED_MEMORY)
            mstore(ALLOCATED_MEMORY, newPointer)
        }
    }

    /**
     * @dev Compute the gas cost of memory expansion.
     * @param words number of words, where a word is 32 bytes
     */
    function memoryExpansionGasCost(uint256 words) internal pure returns (uint256) {
        unchecked {
            return (words.saturatingMul(words) >> 9).saturatingAdd(words.saturatingMul(3));
        }
    }

    /**
     * @dev Compute the amount of gas used by the `GatewayProxy`.
     * @param calldataLen The length of the calldata in bytes
     * @param returnLen The length of the return data in bytes
     */
    function proxyOverheadGasCost(uint256 calldataLen, uint256 returnLen) internal pure returns (uint256) {
        unchecked {
            // Convert the calldata and return data length to words
            calldataLen = _toWord(calldataLen);
            returnLen = _toWord(returnLen);

            // Base cost: OPCODES + COLD SLOAD + COLD DELEGATECALL + RETURNDATACOPY
            // uint256 gasCost = 57 + 2100 + 2600;
            uint256 gasCost = 31 + 2100 + 2600 + 32;

            // CALLDATACOPY
            gasCost = gasCost.saturatingAdd(calldataLen * 3);

            // RETURNDATACOPY
            gasCost = gasCost.saturatingAdd(returnLen * 3);

            // MEMORY EXPANSION (minimal 3 due mstore(0x40, 0x80))
            uint256 words = calldataLen.max(returnLen).max(3);
            gasCost = gasCost.saturatingAdd(memoryExpansionGasCost(words));
            return gasCost;
        }
    }

    /**
     * @dev Compute the gas cost of the `IGateway.submitMessage` method, without the `GatewayProxy` overhead.
     * @param messageSize The size of the message in bytes. This is the `gmp.data.length`.
     */
    function _submitMessageGasCost(uint16 messageSize) private pure returns (uint256 gasCost) {
        unchecked {
            gasCost = SUBMIT_BASE_COST;

            // `countNonZeros` gas cost
            uint256 words = (messageSize + 31) >> 5;
            gasCost += (words * 106) + (((words + 254) / 255) * 214);

            // CALLDATACOPY
            gasCost += words * 3;

            // keccak256 (6 gas per word)
            gasCost += words * 6;

            // emit GmpCreated() gas cost (8 gas per byte)
            gasCost += words << 8;

            // Memory expansion cost
            words += 17 - 1;
            gasCost += ((words * words) >> 9) + (words * 3);

            return gasCost;
        }
    }

    /**
     * @dev Compute the gas cost of the `IGateway.submitMessage` method including the `GatewayProxy` overhead.
     * @param messageSize The size of the message in bytes. This is the `gmp.data.length`.
     */
    function submitMessageGasCost(uint16 messageSize) internal pure returns (uint256 gasCost) {
        unchecked {
            // Compute the gas cost of the `IGateway.submitMessage` method.
            gasCost = _submitMessageGasCost(messageSize);

            // Convert `gmp.data.length` to `abi.encodeCall(IGateway.submitMessage, (...)).length`.
            uint256 calldataSize = ((messageSize + 31) & 0xffe0) + 164;

            // Compute the `GatewayProxy` gas overhead.
            gasCost += proxyOverheadGasCost(uint16(calldataSize), 32);

            return gasCost;
        }
    }

    /**
     * @dev Compute the minimal gas needed to execute the `IGateway.submitMessage` method.
     * @param messageSize The size of the message in bytes. This is the `gmp.data.length`.
     */
    function submitMessageGasNeeded(uint16 messageSize) internal pure returns (uint256 gasNeeded) {
        unchecked {
            // Compute the gas cost of the `IGateway.submitMessage` method.
            gasNeeded = _submitMessageGasCost(messageSize);
            // gasNeeded = gasNeeded.saturatingSub(2114);
            // gasNeeded = _inverseOfAllButOne64th();
            uint256 calldataSize = ((messageSize + 31) & 0xffe0) + 164;
            gasNeeded = gasNeeded.saturatingAdd(proxyOverheadGasCost(calldataSize, 32));
            gasNeeded = gasNeeded.saturatingSub(36);
        }
    }

    /**
     * @dev Estimate the price in wei for send an GMP message.
     * @param gasPrice The gas price in UFloat9x56 format.
     * @param baseFee The base fee in wei.
     * @param nonZeros The number of non-zero bytes in the gmp data.
     * @param zeros The number of zero bytes in the gmp data.
     * @param gasLimit The message gas limit.
     */
    function estimateWeiCost(UFloat9x56 gasPrice, uint256 baseFee, uint16 nonZeros, uint16 zeros, uint256 gasLimit)
        internal
        pure
        returns (uint256)
    {
        // Add execution cost
        uint256 gasCost = estimateGas(nonZeros, zeros, gasLimit);

        // Calculate the gas cost: gasPrice * gasCost + baseFee
        return UFloatMath.saturatingMul(gasPrice, gasCost).saturatingAdd(baseFee);
    }

    /**
     * @dev Estimate the gas cost of a GMP message.
     * @param dataNonZeros The number of non-zero bytes in the gmp data.
     * @param dataZeros The number of zero bytes in the gmp data.
     * @param gasLimit The message gas limit.
     */
    function estimateGas(uint16 dataNonZeros, uint16 dataZeros, uint256 gasLimit) internal pure returns (uint256) {
        uint256 messageSize = uint256(dataNonZeros) + uint256(dataZeros);
        unchecked {
            // add execution cost
            uint256 gasCost =
                computeExecutionRefund(uint16(BranchlessMath.min(messageSize, type(uint16).max)), gasLimit);
            // add base cost
            gasCost = gasCost.saturatingAdd(21000);

            // calldata zero bytes
            uint256 zeros = 31 + 30 + 12 + 30 + 31 + 30;
            zeros = zeros.saturatingAdd((messageSize.saturatingAdd(31) & 0xffffe0) - uint256(dataZeros));
            gasCost = gasCost.saturatingAdd(zeros.saturatingMul(4));

            // calldata non-zero bytes
            uint256 nonZeros = uint256(dataNonZeros).saturatingAdd(4 + 96 + 1 + 32 + 2 + 20 + 2 + 32 + 32 + 1 + 2);
            gasCost = gasCost.saturatingAdd(nonZeros.saturatingMul(16));

            return gasCost;
        }
    }

    /**
     * @dev Convert byte count to 256bit word count, rounded up.
     */
    function _toWord(uint256 byteCount) private pure returns (uint256 words) {
        assembly {
            words := add(shr(5, byteCount), gt(and(byteCount, 0x1f), 0))
        }
    }

    function _debugExecutionGasCost(uint256 messageSize, uint256 gasUsed) internal pure returns (uint256) {
        unchecked {
            // Selector overhead
            // -- First GAS opcode
            uint256 baseCost = EXECUTION_SELECTOR_OVERHEAD - 9;
            uint256 memoryExpansion = 0x60;
            // -- First GAS opcode

            // all opcodes until message.intoCallback()
            baseCost += 449;

            // -- message.intoCallback() --
            baseCost += 438;
            memoryExpansion = 0x80 + 0x01c4;
            uint256 gas = 0;
            // CALLDATACOPY 3 + (3 * words) + memory_expansion
            baseCost += 3;
            gas = _toWord(messageSize) * 3;
            memoryExpansion += messageSize;
            memoryExpansion = memoryExpansion.align32();

            // opcodes until keccak256
            baseCost += 31;

            // keccak256 30 + 6 gas per word
            baseCost += 30;
            gas = gas.saturatingAdd(_toWord(messageSize) * 6);
            //
            baseCost += 424;
            // -- message.intoCallback() --

            baseCost += 34;

            // -- _verifySignature --
            baseCost += 7933;
            // -- _verifySignature --

            baseCost += 18;

            // _execute
            baseCost += 22551;
            baseCost += 2; // GAS

            baseCost += 97;
            //  ------  CALL ------

            baseCost += 2600;
            gas = gas.saturatingAdd(gasUsed);
            memoryExpansion = (messageSize.align32() + 0x80 + 0x0120 + 164).align32();

            //  ------  CALL ------
            baseCost += 67;
            baseCost += 100; // SLOAD
            baseCost += 69;
            baseCost += 100; // SSTORE

            // -- emit GmpExecuted --
            baseCost += 141;
            memoryExpansion += 0x20; // MSTORE
            baseCost += 24;
            memoryExpansion += 0x20; // MSTORE
            baseCost += 39;
            baseCost += 2387; // LOG4
            baseCost += 26;
            // -- emit GmpExecuted --
            // end _execute

            baseCost += 34;

            // GasUtils.txBaseCost()
            {
                baseCost += 64; // base cost

                // chunk start cost
                baseCost += 66;

                // Selector + Signature + GmpMessage
                uint256 words = messageSize.align32().saturatingAdd(388 + 31) >> 5;
                words = (words * 106) + (((words.saturatingSub(255) + 254) / 255) * 214);
                gas = gas.saturatingAdd(words);

                baseCost += 171; // End countNonZeros
                baseCost += 70; // End txBaseCost
            }
            // end GasUtils.txBaseCost()

            baseCost += 482;
            // ----- GAS -------

            baseCost += 168; // GAS
            baseCost += 6800; // REFUND CALL
            baseCost += 184; // RETURN

            gas = gas.saturatingAdd(baseCost);
            gas = gas.saturatingAdd(memoryExpansionGasCost(_toWord(memoryExpansion)));
            return gas;
        }
    }

    function _executionGasCost(uint256 messageSize, uint256 gasUsed) internal pure returns (uint256) {
        // Safety: The operations below can't overflow because the message size can't be greater than 2**16
        unchecked {
            uint256 gas = _toWord(messageSize) * 3;
            gas = gas.saturatingAdd(_toWord(messageSize) * 6);
            gas = gas.saturatingAdd(gasUsed);
            uint256 memoryExpansion = messageSize.align32() + 0x80 + 0x0120 + 164 + 0x40;
            {
                // Selector + Signature + GmpMessage
                uint256 words = messageSize.align32().saturatingAdd(388 + 31) >> 5;
                words = (words * 106) + (((words.saturatingSub(255) + 254) / 255) * 214);
                gas = gas.saturatingAdd(words);
            }
            gas = gas.saturatingAdd(EXECUTION_BASE_COST);
            gas = gas.saturatingAdd(memoryExpansionGasCost(_toWord(memoryExpansion)));
            return gas;
        }
    }

    /**
     * @dev Compute the inverse of `N - floor(N / 64)` defined by EIP-150, used to
     * compute the gas needed for a transaction.
     */
    function inverseOfAllButOne64th(uint256 x) internal pure returns (uint256 inverse) {
        unchecked {
            // inverse = (x * 64) / 63
            inverse = x.saturatingShl(6).saturatingDiv(63);

            // Subtract 1 if `inverse` is a multiple of 64 and greater than 0
            inverse -= BranchlessMath.toUint(inverse > 0 && (inverse % 64) == 0);
        }
    }

    /**
     * @dev Compute the gas needed for the transaction, this is different from the gas used.
     * `gas needed > gas used` because of EIP-150
     */
    function executionGasNeeded(uint256 messageSize, uint256 gasLimit) internal pure returns (uint256 gasNeeded) {
        unchecked {
            gasNeeded = _executionGasCost(messageSize, gasLimit);
            gasNeeded = gasNeeded.saturatingAdd(2300 - 184);
            gasNeeded = inverseOfAllButOne64th(gasNeeded);
            messageSize = messageSize.align32().saturatingAdd(388);
            gasNeeded = gasNeeded.saturatingAdd(proxyOverheadGasCost(messageSize, 64));
            // Remove the proxy final overhead, once the message requires (2300 - 184) extra gas.
            gasNeeded = gasNeeded.saturatingSub(38);
        }
    }

    /**
     * @dev Compute the gas that should be refunded to the executor for the execution.
     * @param messageSize The size of the message.
     * @param gasUsed The gas used by the gmp message.
     */
    function computeExecutionRefund(uint16 messageSize, uint256 gasUsed)
        internal
        pure
        returns (uint256 executionCost)
    {
        // Add the base `IExecutor.execute` gas cost.
        executionCost = _executionGasCost(messageSize, gasUsed);

        // Add `GatewayProxy` gas overhead
        unchecked {
            // Safety: The operations below can't overflow because the message size can't be greater than 2**16
            uint256 calldataSize = ((uint256(messageSize) + 31) & 0xffffe0) + 388; // selector + Signature + GmpMessage
            executionCost = executionCost.saturatingAdd(proxyOverheadGasCost(calldataSize, 64));
        }
    }

    /**
     * @dev Count the number of non-zero bytes in a byte sequence from memory.
     * gas cost = 217 + (words * 112) + ((words - 1) * 193)
     */
    function countNonZeros(bytes memory data) internal pure returns (uint256 nonZeros) {
        assembly ("memory-safe") {
            // Efficient algorithm for counting non-zero bytes in parallel
            let size := mload(data)

            // Temporary set the length of the data to zero
            mstore(data, 0)

            nonZeros := 0
            for {
                // 32 byte aligned pointer, ex: if data.length is 54, then `ptr = data + 32`
                let ptr := add(data, and(add(size, 31), 0xffffffe0))
                let end := xor(data, mul(xor(sub(ptr, 480), data), gt(sub(ptr, data), 480)))
            } true { end := xor(data, mul(xor(sub(ptr, 480), data), gt(sub(ptr, data), 480))) } {
                // Normalize and count non-zero bytes in parallel
                let v := 0
                for {} gt(ptr, end) { ptr := sub(ptr, 32) } {
                    let r := mload(ptr)
                    r := or(r, shr(4, r))
                    r := or(r, shr(2, r))
                    r := or(r, shr(1, r))
                    r := and(r, 0x0101010101010101010101010101010101010101010101010101010101010101)
                    v := add(v, r)
                }

                // Sum bytes in parallel
                v := add(v, shr(128, v))
                v := add(v, shr(64, v))
                v := add(v, shr(32, v))
                v := add(v, shr(16, v))
                v := and(v, 0xffff)
                v := add(and(v, 0xff), shr(8, v))
                nonZeros := add(nonZeros, v)

                if eq(ptr, data) { break }
            }

            // Restore the original length of the data
            mstore(data, size)
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

    /**
     * @dev Compute the transaction base cost.
     */
    function txBaseCost() internal pure returns (uint256) {
        unchecked {
            uint256 nonZeros = countNonZerosCalldata(msg.data);
            uint256 zeros = msg.data.length - nonZeros;
            return 21000 + (nonZeros * 16) + (zeros * 4);
        }
    }
}
