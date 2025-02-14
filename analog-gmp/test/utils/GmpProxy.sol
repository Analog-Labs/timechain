// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/utils/GmpProxy.sol)

pragma solidity >=0.8.0;

import {ERC1967} from "../../src/utils/ERC1967.sol";
import {IGmpReceiver} from "../../src/interfaces/IGmpReceiver.sol";
import {IGateway} from "../../src/interfaces/IGateway.sol";
import {BranchlessMath} from "../../src/utils/BranchlessMath.sol";

contract GmpProxy is IGmpReceiver {
    using BranchlessMath for uint256;

    event MessageReceived(GmpMessage msg);

    struct GmpMessage {
        bytes32 source;
        uint16 srcNetwork;
        address dest;
        uint16 destNetwork;
        uint64 gasLimit;
        uint64 nonce;
        bytes data;
    }

    IGateway public immutable GATEWAY;
    uint16 public immutable NETWORK_ID;

    constructor(address gateway) payable {
        GATEWAY = IGateway(gateway);
        NETWORK_ID = GATEWAY.networkId();
    }

    function sendMessage(GmpMessage calldata message) external payable returns (bytes32) {
        uint256 value = address(this).balance.min(msg.value);
        return GATEWAY.submitMessage{value: value}(message.dest, message.destNetwork, message.gasLimit, message.data);
    }

    function onGmpReceived(bytes32 id, uint128, bytes32, uint64, bytes calldata payload) external payable returns (bytes32) {
        // For testing purpose
        // we keep the original struct in payload so we dont depend on OnGmpReceived call since it doesnt provide everything.
        (GmpMessage memory message) = abi.decode(payload, (GmpMessage));
        message.data = payload;

        emit MessageReceived(message);
        return id;
    }
}
