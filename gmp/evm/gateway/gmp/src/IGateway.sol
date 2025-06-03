// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/interfaces/IGateway.sol)

pragma solidity >=0.8.0;

/**
 * @dev Required interface of an Gateway compliant contract
 */
interface IGateway {
    /**
     * @dev New GMP submitted by calling the `submitMessage` method.
     * @param id EIP-712 hash of the `GmpPayload`, which is it's unique identifier
     * @param source sender account, with an extra flag indicating if it is a contract or an EOA
     * @param destinationAddress the target address on the destination chain.
     * @param destinationNetwork the target chain where the contract call will be made.
     * @param gasLimit the gas limit available for the contract call
     * @param nonce Sequence number per sender, used to guarantee each message is unique.
     * @param data message data with no specified format
     */
    event GmpCreated(
        bytes32 indexed id,
        bytes32 indexed source,
        address indexed destinationAddress,
        uint16 destinationNetwork,
        uint64 gasLimit,
        uint64 nonce,
        bytes data
    );

    function networkId() external view returns (uint16);

    /**
     * @notice Estimate the gas cost of execute a GMP message.
     * @dev This function is called on the destination chain before calling the gateway to execute a source contract.
     * @param networkid The target chain where the contract call will be made
     * @param messageSize Message size
     * @param messageSize Message gas limit
     */
    function estimateMessageCost(uint16 networkid, uint16 messageSize, uint64 gasLimit)
        external
        view
        returns (uint256);

    /**
     * @dev Send message from chain A to chain B
     * @param destinationAddress the target address on the destination chain
     * @param destinationNetwork the target chain where the contract call will be made
     * @param executionGasLimit the gas limit available for the contract call
     * @param data message data with no specified format
     */
    function submitMessage(
        address destinationAddress,
        uint16 destinationNetwork,
        uint64 executionGasLimit,
        bytes calldata data
    ) external payable returns (bytes32);
}
