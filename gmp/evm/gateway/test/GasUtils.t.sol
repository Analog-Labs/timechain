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

contract MeasureGas {
    function baseGas(Signature calldata, Batch calldata) external pure returns (uint256) {
        return GasUtils.txBaseGas();
    }
}

contract GasUtilsTest is Test {
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for address;
    using PrimitiveUtils for uint256;

    Gateway internal gateway;
    IGmpReceiver internal receiver;

    constructor() {
        gateway = TestUtils.setupGateway(42);
        receiver = IGmpReceiver(new GasSpender());
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
            source: bytes32(uint256(0xdead_beef)),
            srcNetwork: 42,
            dest: address(receiver),
            destNetwork: 42,
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
        console.log("callGas", gas.gasTotalUsed - gasLimit);
        uint256 balanceAfter = submitter.addr.balance;

        // check message executed
        assertEq(uint256(gateway.messages(gmp.messageId())), uint256(GmpStatus.SUCCESS));
        // check reimbursment
        assertEq(
            balanceAfter - balanceBefore - baseGas - gasLimit, gas.gasTotalUsed - gasLimit, "Balance should not change"
        );

        // execute second signing session
        balanceBefore = submitter.addr.balance;
        vm.prank(submitter.addr);
        gateway.execute(sig, batch);
        gas = vm.lastCallGas();
        console.log("callGas", gas.gasTotalUsed);
        balanceAfter = submitter.addr.balance;

        // check reimbursment
        assertEq(balanceAfter - balanceBefore - baseGas, gas.gasTotalUsed, "Balance should not change");

        // check replay reverts
        vm.expectRevert("batch already executed");
        gateway.execute(sig, batch);
    }

    string path = "gas.csv";

    function writeFile() private {
        vm.writeFile(path, "numMsg, numReg, numUnreg, msgLen, calldataLen, sessionGas, executionGas\n");
    }

    function writeGas(Gas memory gas) private {
        string memory line = string.concat(
            Strings.toString(gas.numMsg),
            ", ",
            Strings.toString(gas.numReg),
            ", ",
            Strings.toString(gas.numUnreg),
            ", ",
            Strings.toString(gas.msgLen),
            ", ",
            Strings.toString(gas.calldataLen),
            ", ",
            Strings.toString(gas.sessionGas),
            ", ",
            Strings.toString(gas.executionGas)
        );
        vm.writeLine(path, line);
    }

    function test_measure_gas() external {
        writeFile();
        Batch memory batch = TestUtils.emptyBatch(0);
        Gas memory gas = TestUtils.measureGas(gateway, batch);
        writeGas(gas);
        batch = TestUtils.registerBatch(1);
        gas = TestUtils.measureGas(gateway, batch);
        writeGas(gas);
        batch = TestUtils.unregisterBatch(2);
        gas = TestUtils.measureGas(gateway, batch);
        writeGas(gas);
        batch = TestUtils.gmpBatch(32);
        gas = TestUtils.measureGas(gateway, batch);
        writeGas(gas);
        batch = TestUtils.gmpBatch(MAX_PAYLOAD_SIZE);
        gas = TestUtils.measureGas(gateway, batch);
        writeGas(gas);
    }
}
