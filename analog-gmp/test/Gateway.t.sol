// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Gateway.t.sol)

pragma solidity >=0.8.0;

import {Test, console, Vm} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {TestUtils, SigningKey, SigningUtils} from "./TestUtils.sol";
import {GasSpender} from "./utils/GasSpender.sol";
import {BaseTest} from "./utils/BaseTest.sol";
import {Gateway, GatewayEIP712} from "../src/Gateway.sol";
import {GatewayProxy} from "../src/GatewayProxy.sol";
import {GasUtils} from "../src/utils/GasUtils.sol";
import {BranchlessMath} from "../src/utils/BranchlessMath.sol";
import {UFloat9x56, UFloatMath} from "../src/utils/Float9x56.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";
import {IGmpReceiver} from "../src/interfaces/IGmpReceiver.sol";
import {IExecutor} from "../src/interfaces/IExecutor.sol";
import {
    GmpMessage,
    UpdateKeysMessage,
    Signature,
    TssKey,
    Network,
    GmpStatus,
    PrimitiveUtils,
    GmpSender,
    GMP_VERSION
} from "../src/Primitives.sol";

contract SigUtilsTest is GatewayEIP712, Test {
    using PrimitiveUtils for GmpMessage;

    constructor() GatewayEIP712(69, address(0)) {}

    function testPayload() public pure {
        GmpMessage memory gmp = GmpMessage({
            source: GmpSender.wrap(0x0),
            srcNetwork: 42,
            dest: address(0x0),
            destNetwork: 69,
            gasLimit: 0,
            nonce: 0,
            data: ""
        });
        bytes32 typedHash = gmp.eip712hash();
        bytes32 expected = keccak256(
            abi.encode(
                GMP_VERSION,
                gmp.source,
                gmp.srcNetwork,
                gmp.dest,
                gmp.destNetwork,
                gmp.gasLimit,
                gmp.nonce,
                keccak256(gmp.data)
            )
        );
        assertEq(typedHash, expected);
    }
}

struct CallOptions {
    address from;
    address to;
    uint256 value;
    uint256 gasLimit;
    uint256 executionCost;
    uint256 baseCost;
}

library GatewayUtils {
    function tryExecute(CallOptions memory ctx, Signature memory signature, GmpMessage memory message)
        internal
        returns (bool success, GmpStatus status, bytes32 result)
    {
        bytes memory encodedCall = abi.encodeCall(IExecutor.execute, (signature, message));
        bytes memory output;
        (ctx.executionCost, ctx.baseCost, success, output) =
            TestUtils.tryExecuteCall(ctx.from, ctx.to, ctx.gasLimit, ctx.value, encodedCall);

        if (success) {
            require(output.length == 64, "unexpected output length for IExecutor.execute method");
            assembly {
                let ptr := add(output, 32)
                status := mload(ptr)
                result := mload(add(ptr, 32))
            }
        } else {
            status = GmpStatus.NOT_FOUND;
            result = bytes32(0);
        }
    }

    function execute(CallOptions memory ctx, Signature memory signature, GmpMessage memory message)
        internal
        returns (GmpStatus status, bytes32 result)
    {
        bytes memory encodedCall = abi.encodeCall(IExecutor.execute, (signature, message));
        (uint256 executionCost, uint256 baseCost, bytes memory output) =
            TestUtils.executeCall(ctx.from, ctx.to, ctx.gasLimit, ctx.value, encodedCall);

        ctx.executionCost = executionCost;
        ctx.baseCost = baseCost;
        if (output.length == 64) {
            assembly {
                let ptr := add(output, 32)
                status := mload(ptr)
                result := mload(add(ptr, 32))
            }
        }
    }

    function submitMessage(CallOptions memory ctx, GmpMessage memory gmp) internal returns (bytes32 result) {
        bytes memory encodedCall =
            abi.encodeCall(IGateway.submitMessage, (gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data));
        (uint256 executionCost, uint256 baseCost, bytes memory output) =
            TestUtils.executeCall(ctx.from, ctx.to, ctx.gasLimit, ctx.value, encodedCall);
        ctx.executionCost = executionCost;
        ctx.baseCost = baseCost;
        if (output.length == 32) {
            assembly {
                result := mload(add(output, 32))
            }
        }
    }

    function computeGmpGasCost(Signature memory signature, GmpMessage memory message)
        internal
        pure
        returns (uint256 baseCost, uint256 executionCost)
    {
        executionCost = GasUtils.computeExecutionRefund(uint16(message.data.length), 0);
        bytes memory encodedCall = abi.encodeCall(IExecutor.execute, (signature, message));
        baseCost = TestUtils.calculateBaseCost(encodedCall);
    }
}

// contract GatewayBase is Test {
contract GatewayTest is BaseTest {
    using PrimitiveUtils for UpdateKeysMessage;
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for GmpSender;
    using PrimitiveUtils for address;
    using GatewayUtils for CallOptions;
    using BranchlessMath for uint256;
    using SigningUtils for SigningKey;

    Gateway internal gateway;

    // Chronicle TSS Secret
    uint256 private constant SECRET = 0x42;
    uint256 private constant SIGNING_NONCE = 0x69;

    // Receiver Contract, the will waste the exact amount of gas you sent to it in the data field
    IGmpReceiver internal receiver;

    // Netowrk ids
    uint16 private constant SRC_NETWORK_ID = 1234;
    uint16 internal constant DEST_NETWORK_ID = 1337;

    address internal constant ADMIN = 0x6f4c950442e1Af093BcfF730381E63Ae9171b87a;

    constructor() {
        VmSafe.Wallet memory admin = vm.createWallet(SECRET);
        assertEq(ADMIN, admin.addr, "admin address mismatch");
        gateway = Gateway(
            payable(address(TestUtils.setupGateway(admin, bytes32(uint256(1234)), SRC_NETWORK_ID, DEST_NETWORK_ID)))
        );
        receiver = IGmpReceiver(new GasSpender());
    }

    function setUp() external view {
        // check block gas limit as gas left
        assertEq(block.gaslimit, 30_000_000);
        assertTrue(gasleft() >= 10_000_000);
    }

    function sign(GmpMessage memory gmp) internal pure returns (Signature memory) {
        bytes32 hash = gmp.eip712hash();
        SigningKey memory signer = TestUtils.createSigner(SECRET);
        (uint256 e, uint256 s) = signer.signPrehashed(hash, SIGNING_NONCE);
        return Signature({xCoord: signer.xCoord(), e: e, s: s});
    }

    function _sortTssKeys(TssKey[] memory keys) private pure {
        // sort keys by xCoord
        for (uint256 i = 0; i < keys.length; i++) {
            for (uint256 j = i + 1; j < keys.length; j++) {
                if (keys[i].xCoord > keys[j].xCoord) {
                    TssKey memory temp = keys[i];
                    keys[i] = keys[j];
                    keys[j] = temp;
                }
            }
        }
    }

    function test_withinSizeLimit() external {
        bytes memory implementationCreationCode =
            abi.encodePacked(type(Gateway).creationCode, abi.encode(DEST_NETWORK_ID, address(gateway)));
        address implementation =
            FACTORY.create2(bytes32(uint256(1337)), implementationCreationCode, abi.encode(DEST_NETWORK_ID));
        assertLt(implementation.code.length, 0x6000, "implementation code length is too large");
    }

    function test_setShards() external {
        TssKey[] memory keys = new TssKey[](10);

        // create random shard keys
        SigningKey memory signer;
        for (uint256 i = 0; i < keys.length; i++) {
            signer = TestUtils.signerFromEntropy(bytes32(i));
            keys[i] = TssKey({yParity: signer.yParity() == 28 ? 3 : 2, xCoord: signer.xCoord()});
        }
        _sortTssKeys(keys);

        // Only admin can set shards keys
        vm.expectRevert("unauthorized");
        gateway.setShards(keys);

        // Set shards keys must work
        vm.prank(ADMIN, ADMIN);
        gateway.setShards(keys);

        // Check shards keys
        TssKey[] memory shards = gateway.shards();
        _sortTssKeys(shards);
        for (uint256 i = 0; i < shards.length; i++) {
            assertEq(shards[i].xCoord, keys[i].xCoord);
            assertEq(shards[i].yParity, keys[i].yParity);
        }

        // // Replace one shard key
        signer = TestUtils.signerFromEntropy(bytes32(uint256(12345)));
        keys[0].xCoord = signer.xCoord();
        keys[0].yParity = signer.yParity() == 28 ? 3 : 2;
        _sortTssKeys(keys);
        vm.prank(ADMIN, ADMIN);
        gateway.setShards(keys);

        // Check shards keys
        shards = gateway.shards();
        _sortTssKeys(shards);
        for (uint256 i = 0; i < shards.length; i++) {
            assertEq(shards[i].xCoord, keys[i].xCoord);
            assertEq(shards[i].yParity, keys[i].yParity);
        }
    }

    function test_shardEvents() external {
        TssKey[] memory keys = new TssKey[](10);

        // create random shard keys
        SigningKey memory signer;
        for (uint256 i = 0; i < keys.length; i++) {
            signer = TestUtils.signerFromEntropy(bytes32(i));
            keys[i] = TssKey({yParity: signer.yParity() == 28 ? 3 : 2, xCoord: signer.xCoord()});
        }
        _sortTssKeys(keys);

        // set shards
        vm.prank(ADMIN, ADMIN);
        vm.expectEmit(false, false, false, true);
        emit IExecutor.ShardsRegistered(keys);
        gateway.setShards(keys);

        // set a shard which is already registered and verify that is does not emit a event.
        vm.prank(ADMIN, ADMIN);
        vm.recordLogs();
        gateway.setShard(keys[0]);
        Vm.Log[] memory entries = vm.getRecordedLogs();
        assertEq(entries.length, 0);

        // Revoke a registered shard thats not registered.
        uint256 unregisteredSignerKey = 11;
        signer = TestUtils.signerFromEntropy(bytes32(unregisteredSignerKey));
        TssKey memory nonRegisteredKey = TssKey({yParity: signer.yParity() == 28 ? 3 : 2, xCoord: signer.xCoord()});
        vm.prank(ADMIN, ADMIN);
        vm.recordLogs();
        gateway.revokeShard(nonRegisteredKey);
        Vm.Log[] memory entries1 = vm.getRecordedLogs();
        assertEq(entries1.length, 0);

        // Revoke a registered shard
        vm.prank(ADMIN, ADMIN);
        TssKey[] memory unregisteredShardKey = new TssKey[](1);
        unregisteredShardKey[0] = keys[0];
        vm.expectEmit(false, false, false, true);
        emit IExecutor.ShardsUnregistered(unregisteredShardKey);
        gateway.revokeShard(keys[0]);

        // Register a revoked shard
        vm.prank(ADMIN, ADMIN);
        vm.expectEmit(false, false, false, true);
        emit IExecutor.ShardsRegistered(unregisteredShardKey);
        gateway.setShard(unregisteredShardKey[0]);

        // Revoke half of the keys and verify event length
        vm.prank(ADMIN, ADMIN);
        uint256 halfKeysLength = keys.length / 2;
        uint256 secondHalfLength = keys.length - halfKeysLength;
        TssKey[] memory firstHalf = new TssKey[](halfKeysLength);
        TssKey[] memory secondHalf = new TssKey[](secondHalfLength);
        for (uint256 i = 0; i < keys.length; i++) {
            if (i < halfKeysLength) {
                firstHalf[i] = keys[i];
            } else {
                secondHalf[i - halfKeysLength] = keys[i];
            }
        }
        vm.expectEmit(false, false, false, true);
        emit IExecutor.ShardsUnregistered(firstHalf);
        gateway.revokeShards(firstHalf);

        // register first half keys and check if the other half is unregistered
        vm.prank(ADMIN, ADMIN);
        vm.expectEmit(false, false, false, true);
        emit IExecutor.ShardsRegistered(firstHalf);
        emit IExecutor.ShardsUnregistered(secondHalf);
        gateway.setShards(firstHalf);
    }

    function test_Receiver() external {
        bytes memory testEncodedCall = abi.encodeCall(
            IGmpReceiver.onGmpReceived,
            (
                0x0000000000000000000000000000000000000000000000000000000000000000,
                1,
                0x0000000000000000000000000000000000000000000000000000000000000000,
                0,
                abi.encode(uint256(1234))
            )
        );
        // Calling the receiver contract directly to make the address warm
        address sender = TestUtils.createTestAccount(10 ether);
        (uint256 gasUsed,, bytes memory output) =
            TestUtils.executeCall(sender, address(receiver), 23_318, 0, testEncodedCall);
        assertEq(gasUsed, 1234);
        assertEq(output.length, 32);
    }

    function test_estimateMessageCost() external {
        vm.txGasPrice(1);
        uint256 cost = gateway.estimateMessageCost(DEST_NETWORK_ID, 96, 100000);
        assertEq(cost, GasUtils.EXECUTION_BASE_COST + 133821);
    }

    function test_checkPayloadSize() external {
        vm.txGasPrice(1);
        address sender = TestUtils.createTestAccount(100 ether);

        // Build and sign GMP message
        GmpMessage memory gmp = GmpMessage({
            source: sender.toSender(false),
            srcNetwork: SRC_NETWORK_ID,
            dest: address(bytes20(keccak256("dummy_address"))),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 0,
            nonce: 0,
            data: new bytes(24576 + 1)
        });

        Signature memory sig = sign(gmp);

        // Calculate memory expansion cost and base cost
        (uint256 baseCost, uint256 executionCost) = GatewayUtils.computeGmpGasCost(sig, gmp);

        // Transaction Parameters
        CallOptions memory ctx = CallOptions({
            from: sender,
            to: address(gateway),
            value: 0,
            gasLimit: GasUtils.executionGasNeeded(gmp.data.length, gmp.gasLimit) + baseCost + 1_000_000,
            executionCost: 0,
            baseCost: 0
        });

        GmpStatus status;
        bytes32 returned;

        // Expect a revert
        vm.expectRevert("msg data too large");
        (status, returned) = ctx.execute(sig, gmp);
        assertLt(ctx.executionCost, executionCost, "revert should use less gas!!");
        assertEq(ctx.baseCost, baseCost, "unexpected base cost");
    }

    /**
     * @dev Test the gas metering for the `execute` function.
     */
    function test_gasMeter(uint16 messageSize) external {
        vm.assume(messageSize <= 0x6000 && messageSize >= 32);
        vm.txGasPrice(1);
        address sender = TestUtils.createTestAccount(100 ether);

        // Build and sign GMP message
        GmpMessage memory gmp = GmpMessage({
            source: sender.toSender(false),
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 1000,
            nonce: 0,
            data: new bytes(messageSize)
        });
        {
            bytes memory gmpData = gmp.data;
            assembly {
                mstore(add(gmpData, 0x20), 1000)
            }
        }
        Signature memory sig = sign(gmp);

        // Calculate memory expansion cost and base cost
        (uint256 baseCost, uint256 executionCost) = GatewayUtils.computeGmpGasCost(sig, gmp);
        executionCost += gmp.gasLimit;

        // Transaction Parameters
        CallOptions memory ctx = CallOptions({
            from: sender,
            to: address(gateway),
            value: 0,
            gasLimit: GasUtils.executionGasNeeded(gmp.data.length, gmp.gasLimit) + baseCost - 1,
            executionCost: 0,
            baseCost: 0
        });

        GmpStatus status;
        bytes32 returned;

        // Expect a revert
        vm.expectRevert();
        (status, returned) = ctx.execute(sig, gmp);

        // Check if the gateway has enough balance to refund the gas
        uint256 gatewayBalance = address(gateway).balance;
        uint256 senderBalance = address(sender).balance;
        assertGe(gatewayBalance, executionCost + baseCost);
        assertGe(senderBalance, ctx.gasLimit + ctx.value);

        // Give sufficient gas
        ctx.gasLimit += 1;
        ctx.executionCost = 0;
        ctx.baseCost = 0;
        (status, returned) = ctx.execute(sig, gmp);

        assertEq(uint256(status), uint256(GmpStatus.SUCCESS), "gmp execution failed");
        assertEq(uint256(returned), gmp.gasLimit, "wrong gmp return value");
        assertEq(ctx.baseCost, baseCost, "ctx.baseCost != baseCost");
        assertEq(ctx.executionCost, executionCost, "ctx.executionCost != executionCost");
        assertEq(gatewayBalance - address(gateway).balance, executionCost + baseCost, "wrong refund amount");
        assertEq(senderBalance, address(sender).balance, "sender balance should not change");
        assertEq(
            ctx.gasLimit - baseCost, GasUtils.executionGasNeeded(gmp.data.length, gmp.gasLimit), "gas needed mismatch"
        );

        // Submit GMP message
        {
            // Calculate the minimal gmp value minus one
            uint256 nonZeros = GasUtils.countNonZeros(gmp.data);
            uint256 zeros = gmp.data.length - nonZeros;
            ctx.value = GasUtils.estimateGas(uint16(nonZeros), uint16(zeros), gmp.gasLimit) - 1;

            // Add sufficient gas
            ctx.gasLimit += gmp.data.length * 8;
        }

        // Must revert if fund are insufficient
        vm.expectRevert("insufficient tx value");
        ctx.submitMessage(gmp);

        {
            bytes memory submitEncoded =
                abi.encodeCall(IGateway.submitMessage, (gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data));
            assertEq(submitEncoded.length, ((gmp.data.length + 31) & 0xffe0) + 164, "wrong encoded length");
        }

        // Must work if the funds are sufficient
        ctx.value += 1;
        ctx.submitMessage(gmp);

        assertEq(
            ctx.executionCost,
            GasUtils.submitMessageGasCost(uint16(gmp.data.length)) - 4500 + 17100,
            "unexpected submit message gas cost"
        );
    }

    function test_submitMessageMeter(uint16 messageSize) external {
        vm.assume(messageSize <= 0x6000);
        vm.txGasPrice(1);
        address sender = TestUtils.createTestAccount(1000 ether);

        // Build and sign GMP message
        GmpMessage memory gmp = GmpMessage({
            source: sender.toSender(false),
            srcNetwork: DEST_NETWORK_ID,
            dest: address(bytes20(keccak256("dummy_address"))),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 0,
            nonce: 0,
            data: new bytes(messageSize)
        });

        // Calculate memory expansion cost and base cost
        uint256 baseCost;
        {
            bytes memory encoded =
                abi.encodeCall(IGateway.submitMessage, (gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data));
            assertEq(encoded.length, ((gmp.data.length + 31) & 0xffe0) + 164, "wrong encoded length");
            baseCost = TestUtils.calculateBaseCost(encoded);
        }

        // Transaction Parameters
        CallOptions memory ctx = CallOptions({
            from: sender,
            to: address(gateway),
            value: 0,
            gasLimit: GasUtils.submitMessageGasNeeded(uint16(gmp.data.length)) + baseCost,
            executionCost: 0,
            baseCost: 0
        });

        // Submit the transaction
        {
            uint256 nonZeros = GasUtils.countNonZeros(gmp.data);
            uint256 zeros = gmp.data.length - nonZeros;
            ctx.value = GasUtils.estimateGas(uint16(nonZeros), uint16(zeros), gmp.gasLimit);
        }

        uint256 snapshot = vm.snapshotState();
        // Must work if the funds and gas limit are sufficient
        bytes32 id = gmp.eip712hash();
        vm.expectEmit(true, true, true, true);
        emit IGateway.GmpCreated(
            id,
            GmpSender.unwrap(gmp.source),
            gmp.dest,
            gmp.destNetwork,
            uint64(gmp.gasLimit),
            uint64(ctx.value),
            gmp.nonce,
            gmp.data
        );
        console.log("expect: ", ctx.value);
        ctx.gasLimit += 17100;
        assertEq(ctx.submitMessage(gmp), id, "unexpected GMP id");

        // Verify the execution cost
        assertEq(
            ctx.executionCost,
            GasUtils.submitMessageGasCost(uint16(gmp.data.length)) + 17100,
            "unexpected submit message gas cost"
        );

        // Must revert if fund are insufficient
        vm.revertToState(snapshot);
        ctx.value -= 1;
        vm.expectRevert("insufficient tx value");
        ctx.submitMessage(gmp);
    }

    function test_refund() external {
        vm.txGasPrice(1);
        GmpSender sender = TestUtils.createTestAccount(100 ether).toSender(false);

        // GMP message gas used
        uint64 gmpGasUsed = 2_000;

        // Build and sign GMP message
        GmpMessage memory gmp = GmpMessage({
            source: sender,
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: gmpGasUsed,
            nonce: 1,
            data: abi.encodePacked(uint256(gmpGasUsed))
        });
        Signature memory sig = sign(gmp);

        // Estimate execution cost
        (uint256 baseCost, uint256 executionCost) = GatewayUtils.computeGmpGasCost(sig, gmp);
        uint256 expectGasUsed = baseCost + executionCost + gmp.gasLimit;

        // Execute GMP message
        uint256 beforeBalance = sender.toAddress().balance;
        {
            CallOptions memory ctx = CallOptions({
                from: sender.toAddress(),
                to: address(gateway),
                value: 0,
                gasLimit: GasUtils.executionGasNeeded(gmp.data.length, gmp.gasLimit) + baseCost,
                executionCost: 0,
                baseCost: 0
            });
            (GmpStatus status, bytes32 returned) = ctx.execute(sig, gmp);
            {
                // Verify the gas cost
                VmSafe.Gas memory gas = vm.lastCallGas();
                assertEq(gas.gasTotalUsed, executionCost + 2000, "unexpected gas used");
            }

            // Verify the GMP message status
            assertEq(uint256(status), uint256(GmpStatus.SUCCESS), "Unexpected GMP status");
            Gateway.GmpInfo memory info = gateway.gmpInfo(gmp.eip712hash());
            assertEq(
                uint256(info.status), uint256(GmpStatus.SUCCESS), "GMP status stored doesn't match the returned status"
            );
            assertEq(returned, bytes32(uint256(gmp.gasLimit)), "unexpected GMP result");

            // Verify the gas cost
            assertEq(ctx.executionCost + ctx.baseCost, expectGasUsed, "unexpected gas used");
            assertEq(ctx.executionCost, executionCost + gmp.gasLimit, "unexpected execution cost");
        }

        // Verify the gas refund
        uint256 afterBalance = sender.toAddress().balance;
        assertEq(beforeBalance, afterBalance, "wrong refund amount");
    }

    function test_ExecuteRevertsWrongNetwork() external {
        vm.txGasPrice(1);
        uint256 amount = 10 ether;
        address sender = TestUtils.createTestAccount(amount * 2);

        GmpMessage memory wrongNetwork = GmpMessage({
            source: sender.toSender(false),
            srcNetwork: SRC_NETWORK_ID,
            dest: address(0x0),
            destNetwork: SRC_NETWORK_ID,
            gasLimit: 1000,
            nonce: 1,
            data: ""
        });
        Signature memory wrongNetworkSig = sign(wrongNetwork);
        CallOptions memory ctx = CallOptions({
            from: sender,
            to: address(gateway),
            value: 0,
            gasLimit: 1_000_000,
            executionCost: 0,
            baseCost: 0
        });
        vm.expectRevert("invalid gmp network");
        ctx.execute(wrongNetworkSig, wrongNetwork);
    }

    function test_ExecuteRevertsBelowGasLimit() external {
        vm.txGasPrice(1);
        GmpSender sender = TestUtils.createTestAccount(100 ether).toSender(false);
        GmpMessage memory gmp = GmpMessage({
            source: sender,
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 100_000,
            nonce: 1,
            data: abi.encode(uint256(100_000))
        });
        Signature memory sig = sign(gmp);

        // Deposit funds
        (uint256 baseCost, uint256 executionCost) = GatewayUtils.computeGmpGasCost(sig, gmp);

        // Execute GMP message
        CallOptions memory ctx = CallOptions({
            from: sender.toAddress(),
            to: address(gateway),
            value: 0,
            gasLimit: baseCost + executionCost,
            executionCost: 0,
            baseCost: 0
        });
        vm.expectRevert("insufficient gas to execute GMP message");
        ctx.execute(sig, gmp);
    }

    function test_executeRevertsAlreadyExecuted() external {
        vm.txGasPrice(1);
        GmpSender sender = TestUtils.createTestAccount(1000 ether).toSender(false);
        GmpMessage memory gmp = GmpMessage({
            source: sender,
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 1000,
            nonce: 1,
            data: abi.encode(uint256(1000))
        });
        Signature memory sig = sign(gmp);

        // Execute GMP message first time
        CallOptions memory ctx = CallOptions({
            from: sender.toAddress(),
            to: address(gateway),
            value: 0,
            gasLimit: 1_000_000,
            executionCost: 0,
            baseCost: 0
        });
        (GmpStatus status, bytes32 result) = ctx.execute(sig, gmp);
        assertEq(uint256(status), uint256(GmpStatus.SUCCESS), "unexpected GMP status");
        assertEq(gmp.gasLimit, uint256(result), "unexpected GMP result");

        // Execute GMP message second time
        vm.expectRevert("message already executed");
        ctx.execute(sig, gmp);
    }

    function test_submitGmpMessage() external {
        vm.txGasPrice(1);
        GmpSender gmpSender = TestUtils.createTestAccount(1000 ether).toSender(false);
        GmpMessage memory gmp = GmpMessage({
            source: gmpSender,
            srcNetwork: DEST_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 100_000,
            nonce: 0,
            data: abi.encodePacked(uint256(100_000))
        });
        bytes32 id = gmp.eip712hash();

        // Check the previous message hash
        assertEq(gateway.nonceOf(gmp.source.toAddress()), 0, "wrong previous message hash");

        CallOptions memory ctx = CallOptions({
            from: gmpSender.toAddress(),
            to: address(gateway),
            value: 0,
            gasLimit: 1_000_000,
            executionCost: 0,
            baseCost: 0
        });

        // Compute GMP message price
        {
            uint16 nonZeros = uint16(GasUtils.countNonZeros(gmp.data));
            uint16 zeros = uint16(gmp.data.length) - nonZeros;
            ctx.value = GasUtils.estimateWeiCost(UFloatMath.ONE, 0, nonZeros, zeros, gmp.gasLimit);
        }

        // Submit message with insufficient funds
        ctx.value -= 1;
        vm.expectRevert("insufficient tx value");
        ctx.submitMessage(gmp);

        // Submit message with sufficient funds
        ctx.value += 1;
        vm.expectEmit(true, true, true, true);
        emit IGateway.GmpCreated(
            id,
            GmpSender.unwrap(gmp.source),
            gmp.dest,
            gmp.destNetwork,
            uint64(gmp.gasLimit),
            uint64(ctx.value),
            gmp.nonce,
            gmp.data
        );
        assertEq(ctx.submitMessage(gmp), id, "unexpected GMP id");

        // Verify the gas cost
        uint256 expectedCost = GasUtils.submitMessageGasCost(uint16(gmp.data.length)) - 6500;
        assertEq(ctx.executionCost, expectedCost + 17100, "unexpected execution gas cost in first call");

        // Now the second GMP message nonce must be equals to previous message nonce + 1.
        gmp.nonce = gateway.nonceOf(gmp.source.toAddress());
        id = gmp.eip712hash();

        // Expect event
        vm.expectEmit(true, true, true, true);
        emit IGateway.GmpCreated(
            id,
            GmpSender.unwrap(gmp.source),
            gmp.dest,
            gmp.destNetwork,
            uint64(gmp.gasLimit),
            uint64(ctx.value),
            gmp.nonce,
            gmp.data
        );
        assertEq(ctx.submitMessage(gmp), id, "unexpected GMP id");
        assertEq(ctx.executionCost, expectedCost - 6800, "unexpected execution gas cost in second call");
    }
}
