// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/TestUtils.sol)

pragma solidity >=0.8.0;

import {VmSafe, Vm} from "forge-std/Vm.sol";
import {console} from "forge-std/console.sol";
import {Signer} from "frost-evm/Signer.sol";
import {Gateway} from "../src/Gateway.sol";
import {GasUtils} from "../src/GasUtils.sol";
import {GasSpender} from "./GasSpender.sol";
import {
    GmpMessage,
    GmpStatus,
    Signature,
    TssKey,
    Route,
    PrimitiveUtils,
    Batch,
    GatewayOp,
    Command,
    GMP_VERSION,
    MAX_PAYLOAD_SIZE
} from "../src/Primitives.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract SigningHash {
    using PrimitiveUtils for GmpMessage;

    Gateway immutable gw;

    constructor(address gateway) {
        gw = Gateway(payable(gateway));
    }

    function signingHash(Batch calldata batch) external view returns (bytes32) {
        bytes32 rootHash = bytes32(0);

        GatewayOp[] calldata ops = batch.ops;
        for (uint256 i = 0; i < ops.length; i++) {
            GatewayOp calldata op = ops[i];
            bytes calldata params = op.params;

            bytes32 operationHash;
            if (op.command == Command.GMP) {
                GmpMessage calldata gmp;
                assembly {
                    gmp := add(params.offset, 0x20)
                }
                bytes32 msgId = PrimitiveUtils.messageId(gmp);
                bytes32 dataHash = keccak256(gmp.data);
                operationHash = keccak256(abi.encode(msgId, dataHash));
            } else {
                TssKey calldata tssKey;
                assembly {
                    tssKey := params.offset
                }
                operationHash = PrimitiveUtils.hash(tssKey.yParity, tssKey.xCoord, tssKey.numSessions);
            }
            rootHash = PrimitiveUtils.hash(uint256(rootHash), uint256(op.command), uint256(operationHash));
        }
        rootHash = PrimitiveUtils.hash(batch.version, batch.batchId, uint256(rootHash));
        return keccak256(
            abi.encodePacked("Analog GMP v2", gw.networkId(), bytes32(uint256(uint160(address(gw)))), rootHash)
        );
    }
}

struct Gas {
    uint256 executeGas;
    uint256 reimbursmentGas;
    uint256 baseGas;
}

/**
 * @dev Utilities for testing purposes
 */
library TestUtils {
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for address;
    using PrimitiveUtils for uint256;

    // Cheat code address, 0x7109709ECfa91a80626fF3989D68f67F5b1DD12D.
    address internal constant VM_ADDRESS = address(uint160(uint256(keccak256("hevm cheat code"))));
    Vm internal constant vm = Vm(VM_ADDRESS);

    uint256 internal constant admin = uint256(keccak256("admin"));
    uint256 internal constant shard1 = uint256(keccak256("shard1"));
    uint256 internal constant shard2 = uint256(keccak256("shard2"));

    function setupGateway(uint16 network) internal returns (Gateway gateway) {
        VmSafe.Wallet memory _admin = vm.createWallet(admin);
        vm.deal(_admin.addr, 10 ether);
        vm.startPrank(_admin.addr);

        // deploy
        gateway = new Gateway();
        bytes memory initData = abi.encodeWithSelector(Gateway.initialize.selector, network);
        ERC1967Proxy proxy = new ERC1967Proxy(address(gateway), initData);
        console.log("Implementation:", address(gateway));
        console.log("Proxy:", address(proxy));
        console.log("Admin:", _admin.addr);
        vm.deal(address(proxy), 10 ether);
        gateway = Gateway(payable(address(proxy)));

        // register shards
        TssKey[] memory keys = new TssKey[](2);
        Signer signer = new Signer(shard1);
        keys[0] = TssKey({xCoord: signer.xCoord(), yParity: signer.yParity(), numSessions: 1});
        signer = new Signer(shard2);
        keys[1] = TssKey({xCoord: signer.xCoord(), yParity: signer.yParity(), numSessions: 2});
        gateway.setShards(keys, new TssKey[](0));

        // register routes
        gateway.setRoute(
            Route({
                networkId: network,
                gasLimit: 1_000_000,
                baseFee: 0,
                gateway: address(gateway).toSender(),
                relativeGasPriceNumerator: 1,
                relativeGasPriceDenominator: 1,
                gasCoef0: 100000,
                gasCoef1: 10
            })
        );

        vm.stopPrank();
    }

    function prankAdmin() internal {
        VmSafe.Wallet memory _admin = vm.createWallet(admin);
        vm.prank(_admin.addr);
    }

    function msgOp(GmpMessage memory gmp) internal pure returns (GatewayOp memory) {
        return GatewayOp({command: Command.GMP, params: abi.encode(gmp)});
    }

    function registerOp(TssKey memory key) internal pure returns (GatewayOp memory) {
        return GatewayOp({command: Command.RegisterShard, params: abi.encode(key)});
    }

    function unregisterOp(TssKey memory key) internal pure returns (GatewayOp memory) {
        return GatewayOp({command: Command.UnregisterShard, params: abi.encode(key)});
    }

    function makeBatch(uint64 batch, GmpMessage memory gmp) internal pure returns (Batch memory) {
        return TestUtils.makeBatch(batch, TestUtils.msgOp(gmp));
    }

    function makeBatch(uint64 batch, GatewayOp memory op) internal pure returns (Batch memory) {
        GatewayOp[] memory ops = new GatewayOp[](1);
        ops[0] = op;
        return TestUtils.makeBatch(batch, ops);
    }

    function makeBatch(uint64 batch, GatewayOp[] memory ops) internal pure returns (Batch memory) {
        return Batch({version: GMP_VERSION, batchId: batch, ops: ops});
    }

    function sign(uint256 shard, bytes32 hash) internal returns (Signature memory sig) {
        console.log("signing");
        console.logBytes32(hash);
        Signer signer = new Signer(shard);
        (uint256 e, uint256 s) = signer.signPrehashed(uint256(hash), 42);
        return Signature({xCoord: signer.xCoord(), e: e, s: s});
    }

    function sign(uint256 shard, Gateway gw, Batch memory batch) internal returns (Signature memory sig) {
        SigningHash hasher = new SigningHash(address(gw));
        bytes32 hash = hasher.signingHash(batch);
        return TestUtils.sign(shard, hash);
    }

    function calcBaseGas(uint16 messageSize) internal pure returns (uint256) {
        uint256 calldataSize = GasUtils.calldataSize(messageSize);
        return 21000 + calldataSize * 16; // assume every byte is a 1
    }

    function measureGas(uint16 messageSize) internal returns (Gas memory) {
        Gateway gateway = TestUtils.setupGateway(42);
        bytes memory data = new bytes(messageSize);
        assembly {
            mstore(add(data, 32), 5000)
        }
        GmpMessage memory gmp = GmpMessage({
            source: address(0xdead_beef).toSender(),
            srcNetwork: 42,
            dest: address(new GasSpender()),
            destNetwork: 42,
            gasLimit: 5000,
            nonce: 0,
            data: data
        });
        Batch memory batch = TestUtils.makeBatch(uint64(messageSize), gmp);
        Signature memory sig = TestUtils.sign(shard2, gateway, batch);

        gateway.execute(sig, batch);
        uint256 gasUsed = vm.lastCallGas().gasTotalUsed;
        require(uint256(gateway.messages(gmp.messageId())) == uint256(GmpStatus.SUCCESS), "message failed");

        gateway.execute(sig, batch);
        uint256 gasUsed2 = vm.lastCallGas().gasTotalUsed;
        return Gas({
            executeGas: gasUsed - gmp.gasLimit,
            reimbursmentGas: gasUsed2 - gmp.gasLimit,
            baseGas: calcBaseGas(messageSize)
        });
    }
}
