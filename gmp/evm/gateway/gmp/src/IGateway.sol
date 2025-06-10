// SPDX-License-Identifier: MIT

pragma solidity >=0.8.0;

/**
 * @dev Required interface of a Gateway compliant contract
 */
interface IGateway {
    function networkId() external view returns (uint16);

    /**
     * @notice Estimate gas cost of GMP message execution.
     * @dev This function is called on the destination chain before calling the gateway to execute a source contract.
     * @param networkid The target chain where the contract call will be made
     * @param messageSize Message size
     * @param messageSize Message gas limit
     */
    function estimateMessageCost(uint16 networkid, uint256 messageSize, uint64 gasLimit)
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
