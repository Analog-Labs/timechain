// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/MockERC20.sol)

pragma solidity >=0.8.0;

import {ERC20} from "@solmate/tokens/ERC20.sol";
import {IGmpReceiver} from "../src/interfaces/IGmpReceiver.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";

contract MockERC20 is ERC20, IGmpReceiver {
    IGateway private immutable _gateway;
    MockERC20 private immutable _recipientErc20;
    uint16 private immutable _recipientNetwork;

    // Gas limit used to execute `onGmpReceived` method.
    uint256 private constant MSG_GAS_LIMIT = 100_000;

    /**
     * @dev Struct to represent a cross-chain transfer message.
     * @param from The sender address.
     * @param to The recipient address.
     * @param amount The amount of tokens to teleport.
     */
    struct CrossChainTransfer {
        address from;
        address to;
        uint256 amount;
    }

    constructor(
        string memory name,
        string memory symbol,
        IGateway gatewayAddress,
        MockERC20 recipient,
        uint16 recipientNetwork,
        address holder,
        uint256 initialSupply
    ) ERC20(name, symbol, 10) {
        _gateway = gatewayAddress;
        _recipientErc20 = recipient;
        _recipientNetwork = recipientNetwork;
        if (initialSupply > 0) {
            _mint(holder, initialSupply);
        }
    }

    /**
     * @dev Estimate the cost of teleporting tokens to another network.
     */
    function teleportCost() external view returns (uint256) {
        // Estimate the cost
        return _gateway.estimateMessageCost(_recipientNetwork, 96, MSG_GAS_LIMIT);
    }

    /**
     * @dev Teleport tokens to from this contract to another contract on a different network.
     * IMPORTANT: the caller is responsible to compute the teleport cost and send the required amount of ETH.
     * The teleport cost can be computed using the `teleportCost` method.
     *
     * @param to The recipient address on the destination network.
     * @param amount The amount of tokens to teleport.
     */
    function teleport(address to, uint256 amount) external payable returns (bytes32) {
        // Encode the message
        bytes memory message = abi.encode(CrossChainTransfer({from: msg.sender, to: to, amount: amount}));

        // Burn the tokens
        _burn(msg.sender, amount);

        // Submit the GMP message
        return _gateway.submitMessage{value: msg.value}(
            address(_recipientErc20), _recipientNetwork, MSG_GAS_LIMIT, message
        );
    }

    function onGmpReceived(bytes32 id, uint128 network, bytes32 sender, bytes calldata data)
        external
        payable
        returns (bytes32)
    {
        require(msg.sender == address(_gateway), "Unauthorized: only the gateway can call this method");
        require(network == _recipientNetwork, "Unauthorized network");
        require(address(uint160(uint256(sender))) == address(_recipientErc20), "Unauthorized sender");
        CrossChainTransfer memory message = abi.decode(data, (CrossChainTransfer));
        _mint(message.to, message.amount);
        return id;
    }
}
