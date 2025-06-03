// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Gateway.t.sol)

pragma solidity >=0.8.0;

import {Test, console, Vm} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {Signer} from "frost-evm/Signer.sol";
import {TestUtils} from "./TestUtils.sol";
import {GasSpender} from "./GasSpender.sol";
import {Gateway} from "../src/Gateway.sol";
import {GasUtils} from "../src/GasUtils.sol";
import {ShardStore} from "../src/storage/Shards.sol";
import {IGateway} from "gmp/IGateway.sol";
import {IGmpReceiver} from "gmp/IGmpReceiver.sol";
import {
    Batch,
    GmpMessage,
    Signature,
    TssKey,
    GmpCallback,
    GmpStatus,
    PrimitiveUtils,
    GMP_VERSION
} from "../src/Primitives.sol";
import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract TestGatewayV2 is Gateway {
    string public constant VERSION = "v2.0";
    uint256 public newFeature;

    function setNewFeature(uint256 _val) external onlyOwner {
        newFeature = _val;
    }

    function getNewFeature() external view onlyOwner returns (uint256) {
        return newFeature;
    }
}

contract GatewayTest is Test {
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for address;
    using PrimitiveUtils for bytes32;
    using PrimitiveUtils for uint256;

    uint16 internal constant DEST_NETWORK_ID = 1337;
    Gateway internal gateway;
    IGmpReceiver internal receiver;

    constructor() {
        gateway = TestUtils.setupGateway(DEST_NETWORK_ID);
        receiver = IGmpReceiver(new GasSpender());
    }

    function testMessageId() public pure {
        GmpMessage memory gmp = GmpMessage({
            source: address(0x1).toSender(),
            srcNetwork: 42,
            dest: address(0x1),
            destNetwork: 69,
            gasLimit: 0,
            nonce: 2,
            data: "42"
        });
        bytes32 msgId = keccak256(
            abi.encode(GMP_VERSION, gmp.source, gmp.srcNetwork, gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.nonce)
        );
        assertEq(gmp.messageId(), msgId);
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

    function test_setShards() external {
        // start without any shards
        TssKey[] memory shards = gateway.shards();
        TestUtils.prankAdmin();
        gateway.setShards(new TssKey[](0), shards);

        TssKey[] memory keys = new TssKey[](10);

        // create random shard keys
        Signer signer;
        for (uint256 i = 0; i < keys.length; i++) {
            signer = new Signer(i + 1);
            keys[i] = TssKey({yParity: signer.yParity(), xCoord: signer.xCoord(), numSessions: 1});
        }
        _sortTssKeys(keys);

        // Only admin can set shards keys
        address notAdmin = address(0x0000000000000000000000000000000000000000);
        vm.prank(notAdmin);
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, notAdmin));
        gateway.setShards(keys, new TssKey[](0));

        // Set shards keys must work
        TestUtils.prankAdmin();
        gateway.setShards(keys, new TssKey[](0));

        // Check shards keys
        shards = gateway.shards();
        _sortTssKeys(shards);
        for (uint256 i = 0; i < shards.length; i++) {
            assertEq(shards[i].xCoord, keys[i].xCoord);
            assertEq(shards[i].yParity, keys[i].yParity);
        }

        // // Replace one shard key
        signer = new Signer(12345);
        TssKey memory newKey = TssKey({ xCoord: signer.xCoord(), yParity: signer.yParity(), numSessions: 1 });
        TssKey[] memory register = new TssKey[](1);
        register[0] = newKey;
        TssKey[] memory revoke = new TssKey[](1);
        revoke[0] = keys[0];

        TestUtils.prankAdmin();
        gateway.setShards(register, revoke);

        keys[0] = newKey;
        _sortTssKeys(keys);

        // Check shards keys
        shards = gateway.shards();
        _sortTssKeys(shards);
        for (uint256 i = 0; i < shards.length; i++) {
            assertEq(shards[i].xCoord, keys[i].xCoord);
            assertEq(shards[i].yParity, keys[i].yParity);
        }
    }

    function test_shardEvents() external {
        TssKey[] memory keys = new TssKey[](2);

        // create random shard keys
        Signer signer;
        for (uint256 i = 0; i < keys.length; i++) {
            signer = new Signer(i + 1);
            keys[i] = TssKey({yParity: signer.yParity(), xCoord: signer.xCoord(), numSessions: 1});
        }

        // set shards
        TestUtils.prankAdmin();
        vm.expectEmit(false, false, false, true);
        emit ShardStore.ShardRegistered(keys[0]);
        gateway.setShards(keys, new TssKey[](0));

        // set a shard which is already registered and verify that is does not emit a event.
        TestUtils.prankAdmin();
        vm.recordLogs();
        gateway.setShards(keys, new TssKey[](0));
        Vm.Log[] memory entries = vm.getRecordedLogs();
        assertEq(entries.length, 0);

        // Revoke half of the keys and verify event length
        TestUtils.prankAdmin();
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
        vm.recordLogs();
        vm.expectEmit(false, false, false, true);
        emit ShardStore.ShardRevoked(firstHalf[0]);
        gateway.setShards(new TssKey[](0), firstHalf);

        // register first half keys and check if the other half is unregistered
        TestUtils.prankAdmin();
        vm.expectEmit(false, false, false, true);
        emit ShardStore.ShardRegistered(firstHalf[0]);
        emit ShardStore.ShardRevoked(secondHalf[0]);
        gateway.setShards(firstHalf, secondHalf);
    }

    function test_checkPayloadSize() external {
        vm.txGasPrice(1);
        address sender = address(0xdead_beef);
        vm.deal(sender, 10 ether);

        // Build and sign GMP message
        GmpMessage memory gmp = GmpMessage({
            source: sender.toSender(),
            srcNetwork: DEST_NETWORK_ID,
            dest: address(bytes20(keccak256("dummy_address"))),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 0,
            nonce: 0,
            data: new bytes(24576 + 1)
        });
        Batch memory batch = TestUtils.makeBatch(0, gmp);
        Signature memory sig = TestUtils.sign(TestUtils.shard1, gateway, batch);

        // Expect a revert
        vm.expectRevert("msg data too large");
        gateway.execute{gas: 1_000_000}(sig, batch);
    }

    function test_ExecuteRevertsWrongNetwork() external {
        vm.txGasPrice(1);
        address sender = address(0xdead_beef);
        vm.deal(sender, 10 ether);

        GmpMessage memory wrongNetwork = GmpMessage({
            source: sender.toSender(),
            srcNetwork: 42,
            dest: address(0x0),
            destNetwork: 42,
            gasLimit: 1000,
            nonce: 1,
            data: ""
        });
        Batch memory batch = TestUtils.makeBatch(0, wrongNetwork);
        Signature memory wrongNetworkSig = TestUtils.sign(TestUtils.shard1, gateway, batch);
        vm.startPrank(sender);
        vm.expectRevert("invalid gmp network");
        gateway.execute{gas: 1_000_000}(wrongNetworkSig, batch);
        vm.stopPrank();
    }

    function test_ExecuteRevertsBelowGasLimit() external {
        vm.txGasPrice(1);
        address sender = address(0xdead_beef);
        vm.deal(sender, 10 ether);
        GmpMessage memory gmp = GmpMessage({
            source: sender.toSender(),
            srcNetwork: DEST_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 100_000,
            nonce: 1,
            data: abi.encode(uint256(100_000))
        });
        Batch memory batch = TestUtils.makeBatch(0, gmp);
        Signature memory sig = TestUtils.sign(TestUtils.shard1, gateway, batch);

        // Execute GMP message
        vm.expectRevert("insufficient gas to execute GMP message");
        gateway.execute{gas: 100_000}(sig, batch);
    }

    function test_submitGmpMessage() external {
        vm.txGasPrice(1);
        address sender = address(0xdead_beef99);
        vm.deal(sender, 10 ether);
        GmpMessage memory gmp = GmpMessage({
            source: sender.toSender(),
            srcNetwork: DEST_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 100_000,
            nonce: 0,
            data: abi.encodePacked(uint256(100_000))
        });
        bytes32 id = gmp.messageId();

        // Check the previous message hash
        assertEq(gateway.nonces(gmp.source.toAddress()), 0, "wrong previous message hash");

        // Compute GMP message price
        uint256 value = gateway.estimateMessageCost(DEST_NETWORK_ID, uint16(gmp.data.length), gmp.gasLimit);
        console.log("messageSize", gmp.data.length);
        console.log("gasLimit", gmp.gasLimit);
        console.log("messageCost", value);

        // Submit message with insufficient funds
        vm.expectRevert("insufficient tx value");
        gateway.submitMessage{value: value - 1}(gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data);

        // Submit message with sufficient funds
        vm.expectEmit(true, true, true, true);
        emit IGateway.GmpCreated(
            id, gmp.source, gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.nonce, gmp.data
        );
        vm.startPrank(sender);
        bytes32 rid = gateway.submitMessage{value: value}(gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data);
        vm.stopPrank();
        assertEq(rid, id, "unexpected GMP id");

        // Now the second GMP message nonce must be equals to previous message nonce + 1.
        gmp.nonce = gateway.nonces(gmp.source.toAddress());
        id = gmp.messageId();

        // Expect event
        vm.expectEmit(true, true, true, true);
        emit IGateway.GmpCreated(
            id, gmp.source, gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.nonce, gmp.data
        );
        vm.startPrank(sender);
        rid = gateway.submitMessage{value: value}(gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data);
        vm.stopPrank();
        assertEq(rid, id, "unexpected GMP id");
    }

    function test_upgradeOnlyAdmin() public {
        TestGatewayV2 gatewayv2 = new TestGatewayV2();
        address notAdmin = address(0x0000000000000000000000000000000000000000);
        vm.startPrank(notAdmin);
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, notAdmin));
        gateway.upgradeToAndCall(address(gatewayv2), "");
        vm.stopPrank();
        TestUtils.prankAdmin();
        gateway.upgradeToAndCall(address(gatewayv2), "");
    }

    function test_storagePreservationAfterUpgrade() public {
        uint256 initialNetworkId = gateway.networkId();

        TestGatewayV2 gatewayV2 = new TestGatewayV2();
        TestUtils.prankAdmin();
        gateway.upgradeToAndCall(address(gatewayV2), "");
        assertEq(gateway.networkId(), initialNetworkId, "Network ID changed");
    }

    function test_newFeatureAfterUpgrade() public {
        TestGatewayV2 gatewayV2 = new TestGatewayV2();
        TestUtils.prankAdmin();
        gateway.upgradeToAndCall(address(gatewayV2), "");

        TestGatewayV2 upgraded = TestGatewayV2(address(gateway));

        address notAdmin = address(0x0000000000000000000000000000000000000000);
        vm.prank(notAdmin);
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, notAdmin));
        upgraded.setNewFeature(100);

        TestUtils.prankAdmin();
        uint256 newFeature = 100;
        upgraded.setNewFeature(newFeature);
        TestUtils.prankAdmin();
        uint256 receivedFeature = upgraded.getNewFeature();
        assertEq(newFeature, receivedFeature);
    }
}
