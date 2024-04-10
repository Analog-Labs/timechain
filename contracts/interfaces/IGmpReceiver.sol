// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (contracts/interfaces/IGmpReceiver.sol)

pragma solidity ^0.8.20;

/**
 * @dev Required interface for contracts that receive GMP messages
 */
interface IGmpReceiver {
    /**
     * @dev Handles the receipt of a single GMP message.
     * The contract must verify the msg.sender, it must be the Gateway Contract address.
     *
     * @param id The EIP-712 hash of the message payload, used as GMP unique identifier
     * @param network The chain_id of the source chain who send the message
     * @param source The pubkey/address which sent the GMP message
     * @param payload The message payload with no specified format
     * @return 32 byte result which will be stored together with GMP message
     */
    function onGmpReceived(bytes32 id, uint128 network, bytes32 source, bytes calldata payload)
        external
        payable
        returns (bytes32);
}
