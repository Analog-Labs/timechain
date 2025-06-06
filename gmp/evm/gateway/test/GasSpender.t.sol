// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Gateway.t.sol)

pragma solidity >=0.8.0;

import {Test} from "forge-std/Test.sol";
import {GasSpender} from "./GasSpender.sol";
import {IGmpReceiver} from "gmp/IGmpReceiver.sol";

contract GasSpenderTest is Test {
    IGmpReceiver internal receiver;

    constructor() {
        receiver = IGmpReceiver(new GasSpender());
    }

    function test_onGmpReceivedWorks(uint16 delta) external {
        uint256 gasToWaste = 1000 + uint256(delta);
        receiver.onGmpReceived{gas: gasToWaste}(0x0, 1, 0x0, 0, abi.encode(gasToWaste));
        uint256 gasUsed = vm.lastCallGas().gasTotalUsed;
        assertEq(gasUsed, gasToWaste);
    }

    function test_revertsMoreGas(uint16 delta) external {
        uint256 gasToWaste = 1000 + uint256(delta);
        vm.expectRevert();
        receiver.onGmpReceived{gas: gasToWaste + 1}(0x0, 1, 0x0, 0, abi.encode(gasToWaste));
    }

    function test_revertsLessGas(uint16 delta) external {
        uint256 gasToWaste = 1000 + uint256(delta);
        vm.expectRevert();
        receiver.onGmpReceived{gas: gasToWaste - 1}(0x0, 1, 0x0, 0, abi.encode(gasToWaste));
    }
}
