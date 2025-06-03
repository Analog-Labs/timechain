// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/utils/GmpProxy.sol)

pragma solidity >=0.8.0;

import {IGmpReceiver} from "gmp/IGmpReceiver.sol";
import {IGateway} from "gmp/IGateway.sol";
import {console} from "forge-std/console.sol";

contract GmpProxy is IGmpReceiver {
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
        return
            GATEWAY.submitMessage{value: msg.value}(message.dest, message.destNetwork, message.gasLimit, message.data);
    }

    function onGmpReceived(bytes32 id, uint128 srcNetwork, bytes32 src, uint64 nonce, bytes calldata payload)
        external
        payable
        returns (bytes32)
    {
        // when estimating gas an insane amount of gas is provided
        uint256 gasLimit = gasleft();
        // this is the constant added to gasLimit
        unchecked {
            console.log(300_000 - gasLimit);
        }
        uint64 msgGasLimit;
        unchecked {
            msgGasLimit = uint64(gasLimit + 579);
        }
        GmpMessage memory message = GmpMessage({
            source: src,
            srcNetwork: uint16(srcNetwork),
            dest: address(this),
            destNetwork: NETWORK_ID,
            gasLimit: msgGasLimit,
            nonce: nonce,
            data: payload
        });
        emit MessageReceived(message);
        return id;
    }
}
