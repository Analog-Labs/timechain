// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Gateway.t.sol)

pragma solidity >=0.8.0;

import {Test, console, Vm} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {TestUtils} from "../TestUtils.sol";
import {BaseTest} from "./BaseTest.sol";
import {GasSpender} from "./GasSpender.sol";
import {GasUtils} from "../../src/utils/GasUtils.sol";
import {IGmpReceiver} from "../../src/interfaces/IGmpReceiver.sol";

contract GasSpenderTest is BaseTest {
    function buildCall(uint256 gasToWaste) private pure returns (uint256 gasLimit, bytes memory encodedCall) {
        // Encode the `IGmpReceiver.onGmpReceived` call
        encodedCall = abi.encodeCall(
            IGmpReceiver.onGmpReceived,
            (
                0x0000000000000000000000000000000000000000000000000000000000000000,
                1,
                0x0000000000000000000000000000000000000000000000000000000000000000,
                0,
                abi.encode(gasToWaste)
            )
        );
        gasLimit = TestUtils.calculateBaseCost(encodedCall) + gasToWaste;
    }

    function test_onGmpReceivedWorks(uint16 delta) external {
        // Guarantee the gas limit is not less than 1000
        uint256 gasToWaste = 1000 + uint256(delta);
        vm.txGasPrice(1);

        // Create the Sender account
        address sender = TestUtils.createTestAccount(100 ether);

        // Deploy the GasSpender contract
        GasSpender spender = new GasSpender();

        // Encode the `IGmpReceiver.onGmpReceived` call
        (uint256 gasLimit, bytes memory encodedCall) = buildCall(gasToWaste);

        (uint256 gasUsed,, bytes memory output) =
            TestUtils.executeCall(sender, address(spender), gasLimit, 0, encodedCall);
        assertEq(gasUsed, gasToWaste);
        assertEq(output.length, 32);
    }

    function test_revertsMoreGas(uint16 delta) external {
        // Guarantee the gas limit is not less than 1000
        uint256 gasToWaste = 1000 + uint256(delta);
        vm.txGasPrice(1);

        // Create the Sender account
        address sender = TestUtils.createTestAccount(100 ether);

        // Deploy the GasSpender contract
        GasSpender spender = new GasSpender();

        // Encode the `IGmpReceiver.onGmpReceived` call
        (uint256 gasLimit, bytes memory encodedCall) = buildCall(gasToWaste);

        vm.expectRevert();
        TestUtils.executeCall(sender, address(spender), gasLimit + 1, 0, encodedCall);
    }

    function test_revertsLessGas(uint16 delta) external {
        // Guarantee the gas limit is not less than 1000
        uint256 gasToWaste = 1000 + uint256(delta);
        vm.txGasPrice(1);

        // Create the Sender account
        address sender = TestUtils.createTestAccount(100 ether);

        // Deploy the GasSpender contract
        GasSpender spender = new GasSpender();

        // Encode the `IGmpReceiver.onGmpReceived` call
        (uint256 gasLimit, bytes memory encodedCall) = buildCall(gasToWaste);

        vm.expectRevert();
        TestUtils.executeCall(sender, address(spender), gasLimit - 1, 0, encodedCall);
    }
}
