// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/GasUtils.t.sol)

pragma solidity >=0.8.0;

import {Test, console} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";
import {Gas, TestUtils} from "./TestUtils.sol";
import {GasSpender} from "./GasSpender.sol";
import {Gateway} from "../src/Gateway.sol";
import {GasUtils} from "../src/GasUtils.sol";
import {IGmpReceiver} from "gmp/IGmpReceiver.sol";
import {
    Command,
    GatewayOp,
    GmpMessage,
    Signature,
    TssKey,
    GmpStatus,
    PrimitiveUtils,
    Batch,
    GMP_VERSION,
    MAX_PAYLOAD_SIZE
} from "../src/Primitives.sol";

uint256 constant secret = 0x42;
uint256 constant nonce = 0x69;

contract MeasureGas {
    function baseGas(Signature calldata, Batch calldata) external pure returns (uint256) {
        return GasUtils.txBaseGas();
    }

    function proxyOverheadGas(Signature calldata, Batch calldata) external pure returns (uint256) {
        return GasUtils.proxyOverheadGas(msg.data.length);
    }
}

contract GasUtilsTest is Test {
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for address;
    using PrimitiveUtils for uint256;

    Gateway internal gateway;
    IGmpReceiver internal receiver;

    bytes32 private constant SENDER = bytes32(uint256(0xdead_beef));
    uint16 private constant SRC_NETWORK_ID = 1234;
    uint16 private constant DEST_NETWORK_ID = 1337;

    string path = "gas.csv";

    constructor() {
        gateway = TestUtils.setupGateway(DEST_NETWORK_ID);
        receiver = IGmpReceiver(new GasSpender());
        vm.writeFile(path, "messageSize, executeGas, reimbursmentGas, baseGas\n");
    }

    function test_calldata_size(uint16 messageSize) external {
        bytes memory data = new bytes(messageSize);
        GmpMessage memory gmp = GmpMessage({
            source: SENDER,
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 42,
            nonce: 42,
            data: data
        });
        Batch memory batch = TestUtils.makeBatch(1, gmp);
        Signature memory sig = TestUtils.sign(TestUtils.shard1, gateway, batch);
        bytes memory call = abi.encodeCall(gateway.execute, (sig, batch));
        assertEq(call.length, GasUtils.calldataSize(messageSize));
    }

    /**
     * @dev Compare the estimated gas cost VS the actual gas cost of the `execute` method.
     */
    function test_reimbursment(uint16 messageSize, uint16 gasLimit) external {
        vm.txGasPrice(1);
        vm.assume(gasLimit >= 5000);
        vm.assume(messageSize <= (0x6000 - 32));
        messageSize += 32;

        VmSafe.Wallet memory submitter = vm.createWallet(uint256(keccak256("submitter")));
        vm.deal(submitter.addr, 10 ether);

        bytes memory data = new bytes(messageSize);
        assembly {
            mstore(add(data, 32), gasLimit)
        }
        GmpMessage memory gmp = GmpMessage({
            source: SENDER,
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: gasLimit,
            nonce: gasLimit,
            data: data
        });
        Batch memory batch = TestUtils.makeBatch(0, gmp);
        Signature memory sig = TestUtils.sign(TestUtils.shard2, gateway, batch);

        console.log("messageSize", messageSize);
        console.log("gasLimit", gasLimit);

        MeasureGas m = new MeasureGas();
        uint256 baseGas = m.baseGas(sig, batch);
        console.log("baseGas", baseGas);

        // execute
        uint256 balanceBefore = submitter.addr.balance;
        vm.prank(submitter.addr);
        gateway.execute(sig, batch);
        VmSafe.Gas memory gas = vm.lastCallGas();
        console.log('callGas', gas.gasTotalUsed - gasLimit);
        uint256 balanceAfter = submitter.addr.balance;

        // check message executed
        assertEq(uint256(gateway.messages(gmp.messageId())), uint256(GmpStatus.SUCCESS));
        // check reimbursment
        assertEq(balanceAfter - balanceBefore - baseGas - gasLimit, gas.gasTotalUsed - gasLimit, "Balance should not change");

        // execute second signing session
        balanceBefore = submitter.addr.balance;
        vm.prank(submitter.addr);
        gateway.execute(sig, batch);
        gas = vm.lastCallGas();
        console.log('callGas', gas.gasTotalUsed);
        balanceAfter = submitter.addr.balance;

        // check reimbursment
        assertEq(balanceAfter - balanceBefore - baseGas, gas.gasTotalUsed, "Balance should not change");

        // check replay reverts
        vm.expectRevert("batch already executed");
        gateway.execute(sig, batch);
    }

    function test_measure_gas(uint16 messageSize) external {
        vm.assume(messageSize <= MAX_PAYLOAD_SIZE - 32);
        messageSize += 32;
        Gas memory gas = TestUtils.measureGas(messageSize);

        string memory line = string.concat(
            Strings.toString(messageSize),
            ", ",
            Strings.toString(gas.executeGas),
            ", ",
            Strings.toString(gas.reimbursmentGas),
            ", ",
            Strings.toString(gas.baseGas)
        );
        vm.writeLine(path, line);
    }
}
