// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/GasUtils.t.sol)

pragma solidity >=0.8.0;

import {Signer} from "frost-evm/sol/Signer.sol";
import {Test, console} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {TestUtils} from "./TestUtils.sol";
import {BaseTest} from "./utils/BaseTest.sol";
import {GasSpender} from "./utils/GasSpender.sol";
import {Gateway, GatewayEIP712} from "../src/Gateway.sol";
import {GatewayProxy} from "../src/GatewayProxy.sol";
import {GasUtils} from "../src/utils/GasUtils.sol";
import {BranchlessMath} from "../src/utils/BranchlessMath.sol";
import {UFloat9x56, UFloatMath} from "../src/utils/Float9x56.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";
import {IGmpReceiver} from "../src/interfaces/IGmpReceiver.sol";
import {IExecutor} from "../src/interfaces/IExecutor.sol";
import {CallOptions, GatewayUtils} from "./Gateway.t.sol";
import {
    GmpMessage,
    UpdateKeysMessage,
    Signature,
    TssKey,
    Network,
    GmpStatus,
    PrimitiveUtils,
    GmpSender
} from "../src/Primitives.sol";

uint256 constant secret = 0x42;
uint256 constant nonce = 0x69;

contract GasUtilsMock {
    function execute(Signature calldata, GmpMessage calldata)
        external
        pure
        returns (uint256 baseCost, uint256 nonZeros, uint256 zeros)
    {
        baseCost = GasUtils.txBaseCost();
        nonZeros = GasUtils.countNonZerosCalldata(msg.data);
        zeros = msg.data.length - nonZeros;
    }
}

contract GasUtilsTest is BaseTest {
    using PrimitiveUtils for UpdateKeysMessage;
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for GmpSender;
    using PrimitiveUtils for address;
    using GatewayUtils for CallOptions;
    using BranchlessMath for uint256;

    GasUtilsMock internal mock;
    Gateway internal gateway;
    Signer internal signer;

    // Receiver Contract, the will waste the exact amount of gas you sent to it in the data field
    IGmpReceiver internal receiver;

    uint16 private constant SRC_NETWORK_ID = 1234;
    uint16 internal constant DEST_NETWORK_ID = 1337;

    constructor() {
        TestUtils.deployFactory();

        // Create the Shard and Admin accounts
        signer = new Signer(secret);
        VmSafe.Wallet memory deployer = vm.createWallet(secret);
        vm.deal(deployer.addr, 100 ether);

        // Deploy the GasUtilsMock contract
        mock = new GasUtilsMock();

        // Deploy the GatewayProxy
        gateway = Gateway(
            payable(address(TestUtils.setupGateway(deployer, bytes32(uint256(0)), SRC_NETWORK_ID, DEST_NETWORK_ID)))
        );
        vm.deal(address(gateway), 100 ether);

        // Deploy the GasSpender contract, which implements the IGmpReceiver interface.
        receiver = IGmpReceiver(new GasSpender());
    }

    function sign(GmpMessage memory gmp) internal view returns (Signature memory) {
        uint256 hash = uint256(gmp.eip712hash());
        (uint256 e, uint256 s) = signer.signPrehashed(hash, nonce);
        return Signature({xCoord: signer.xCoord(), e: e, s: s});
    }

    /**
     * @dev Create a GMP message with the provided parameters.
     */
    function _buildGmpMessage(address sender, uint64 gasLimit, uint64 gasUsed, uint256 messageSize)
        private
        view
        returns (GmpMessage memory message, Signature memory signature, CallOptions memory context)
    {
        require(gasUsed == 0 || messageSize >= 32, "If gasUsed > 0, then messageSize must be >= 32");
        require(messageSize <= 0x6000, "message is too big");

        // Setup data and receiver addresses.
        bytes memory data = new bytes(messageSize);
        address gmpReceiver;
        if (gasUsed > 0) {
            gmpReceiver = address(receiver);
            assembly {
                mstore(add(data, 32), gasUsed)
            }
        } else {
            // Create a new unique receiver address for each message, otherwise the gas refund will not work.
            gmpReceiver = address(bytes20(keccak256(abi.encode(sender, gasLimit, messageSize))));
        }

        // Build the GMP message
        message = GmpMessage({
            source: sender.toSender(false),
            srcNetwork: SRC_NETWORK_ID,
            dest: gmpReceiver,
            destNetwork: DEST_NETWORK_ID,
            gasLimit: gasLimit,
            nonce: 0,
            data: data
        });

        // Sign the message
        signature = sign(message);

        // Calculate memory expansion cost and base cost
        (uint256 baseCost, uint256 executionCost) = GatewayUtils.computeGmpGasCost(signature, message);

        // Set Transaction Parameters
        context = CallOptions({
            from: sender,
            to: address(gateway),
            value: 0,
            gasLimit: GasUtils.executionGasNeeded(message.data.length, message.gasLimit).saturatingAdd(baseCost),
            executionCost: executionCost,
            baseCost: baseCost
        });
    }

    /**
     * Test the `GasUtils.txBaseCost` method.
     */
    function test_txBaseCost() external view {
        // Build and sign GMP message
        GmpMessage memory gmp = GmpMessage({
            source: address(0x1111111111111111111111111111111111111111).toSender(false),
            srcNetwork: 1234,
            dest: address(0x2222222222222222222222222222222222222222),
            destNetwork: 1337,
            gasLimit: 0,
            nonce: 0,
            data: hex"00"
        });
        Signature memory sig = sign(gmp);
        sig.xCoord = type(uint256).max;
        sig.e = type(uint256).max;
        sig.s = type(uint256).max;

        // Check if `IExecutor.execute` match the expected base cost
        (uint256 baseCost, uint256 nonZeros, uint256 zeros) = mock.execute(sig, gmp);
        assertEq(baseCost, 24444, "Wrong calldata gas cost");
        assertEq(nonZeros, 147, "wrong number of non-zeros");
        assertEq(zeros, 273, "wrong number of zeros");
    }

    /**
     * @dev Compare the estimated gas cost VS the actual gas cost of the `execute` method.
     */
    function test_baseExecutionCost(uint16 messageSize, uint16 gasLimit) external {
        vm.assume(gasLimit >= 5000);
        vm.assume(messageSize <= (0x6000 - 32));
        messageSize += 32;
        vm.txGasPrice(1);
        address sender = TestUtils.createTestAccount(100 ether);

        // Build the GMP message
        GmpMessage memory gmp;
        Signature memory sig;
        CallOptions memory ctx;
        (gmp, sig, ctx) = _buildGmpMessage(sender, gasLimit, gasLimit, messageSize);

        // Increase the gas limit to avoid out-of-gas errors
        ctx.gasLimit = ctx.gasLimit.saturatingAdd(10_000_000);

        // Execute the GMP message
        {
            bytes32 gmpId = gmp.eip712hash();
            vm.expectEmit(true, true, true, true);
            emit IExecutor.GmpExecuted(gmpId, gmp.source, gmp.dest, GmpStatus.SUCCESS, bytes32(uint256(gasLimit)));
            uint256 balanceBefore = ctx.from.balance;
            (GmpStatus status, bytes32 result) = ctx.execute(sig, gmp);
            assertEq(uint256(status), uint256(GmpStatus.SUCCESS), "GMP execution failed");
            assertEq(result, bytes32(uint256(gasLimit)), "unexpected result");
            assertEq(balanceBefore, ctx.from.balance, "Balance should not change");
        }

        emit log_named_uint("execution cost", GasUtils._executionGasCost(gmp.data.length, gmp.gasLimit));
        uint256 executionCost = GasUtils.computeExecutionRefund(uint16(gmp.data.length), gmp.gasLimit);
        assertEq(ctx.executionCost, executionCost, "execution cost mismatch");

        // Calculate the expected base cost
        uint256 dynamicCost = executionCost - GasUtils.EXECUTION_BASE_COST;
        uint256 expectedBaseCost = ctx.executionCost - dynamicCost;
        {
            console.log("proxy: ", ctx.to);
            console.logBytes(ctx.to.code);
            address implementationAddr = address(
                uint160(uint256(vm.load(ctx.to, 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc)))
            );
            console.log("implementation: ", implementationAddr);
            console.logBytes(implementationAddr.code);
            console.log("calldata:");
            console.logBytes(abi.encodeCall(IExecutor.execute, (sig, gmp)));
        }
        assertEq(expectedBaseCost, GasUtils.EXECUTION_BASE_COST, "Wrong EXECUTION_BASE_COST");
    }

    function test_gasUtils() external pure {
        uint256 baseCost = GasUtils.EXECUTION_BASE_COST;
        assertEq(GasUtils.estimateGas(0, 0, 0), 31528 + baseCost);
        assertEq(GasUtils.estimateGas(0, 33, 0), 31901 + baseCost);
        assertEq(GasUtils.estimateGas(33, 0, 0), 32561 + baseCost);
        assertEq(GasUtils.estimateGas(20, 13, 0), 32301 + baseCost);

        UFloat9x56 one = UFloatMath.ONE;
        assertEq(GasUtils.estimateWeiCost(one, 0, 0, 0, 0), 31528 + baseCost);
        assertEq(GasUtils.estimateWeiCost(one, 0, 0, 33, 0), 31901 + baseCost);
        assertEq(GasUtils.estimateWeiCost(one, 0, 33, 0, 0), 32561 + baseCost);
        assertEq(GasUtils.estimateWeiCost(one, 0, 20, 13, 0), 32301 + baseCost);

        UFloat9x56 two = UFloat9x56.wrap(0x8080000000000000);
        assertEq(GasUtils.estimateWeiCost(two, 0, 0, 0, 0), (31528 + baseCost) * 2);
        assertEq(GasUtils.estimateWeiCost(two, 0, 0, 33, 0), (31901 + baseCost) * 2);
        assertEq(GasUtils.estimateWeiCost(two, 0, 33, 0, 0), (32561 + baseCost) * 2);
        assertEq(GasUtils.estimateWeiCost(two, 0, 20, 13, 0), (32301 + baseCost) * 2);
    }
}
