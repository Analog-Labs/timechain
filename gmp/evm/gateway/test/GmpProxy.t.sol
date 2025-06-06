// SPDX-License-Identifier: MIT

pragma solidity >=0.8.0;

import {Test, console, Vm} from "forge-std/Test.sol";
import {IGateway} from "gmp/IGateway.sol";
import {IGmpReceiver} from "gmp/IGmpReceiver.sol";
import {GmpProxy} from "src/GmpProxy.sol";

contract MockGateway is IGateway {
    uint16 _networkId;

    constructor(uint16 network) {
        _networkId = network;
    }

    function networkId() external view returns (uint16) {
        return _networkId;
    }

    function estimateMessageCost(uint16, uint16, uint64) external pure returns (uint256) {
        return 0;
    }

    function submitMessage(address, uint16, uint64, bytes calldata) external payable returns (bytes32) {
        return 0x0;
    }
}

contract GmpProxyTest is Test {
    MockGateway gateway;
    GmpProxy proxy;

    function setUp() external {
        gateway = new MockGateway(1);
        proxy = new GmpProxy(address(gateway));
    }

    function test_onGmpReceived() external {
        proxy.onGmpReceived{gas: 300000}(0x0, 0, 0x0, 0, abi.encode(0x0));
    }
}
