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
    DOMAIN_SEPARATOR,
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
            abi.encodePacked(DOMAIN_SEPARATOR, gw.networkId(), bytes32(uint256(uint160(address(gw)))), rootHash)
        );
    }
}

struct Gas {
    uint256 numMsg;
    uint256 numReg;
    uint256 numUnreg;
    uint256 msgLen;
    uint256 calldataLen;
    uint256 sessionGas;
    uint256 executionGas;
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
    uint256 internal constant newShard = uint256(keccak256("newShard"));

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
        keys[0] = TestUtils.tssKey(shard1, 1);
        keys[1] = TestUtils.tssKey(shard2, 2);
        gateway.setShards(keys, new TssKey[](0));

        // register routes
        gateway.setRoute(
            Route({
                networkId: network,
                gateway: address(gateway).toSender(),
                maxGasLimit: 1_000_000,
                msgGas: 100_000,
                msgByteGas: 20,
                gasPriceNumerator: 1,
                gasPriceDenominator: 1,
                msgFee: 0
            })
        );

        vm.stopPrank();
    }

    function prankAdmin() internal {
        VmSafe.Wallet memory _admin = vm.createWallet(admin);
        vm.prank(_admin.addr);
    }

    function tssKey(uint256 privateKey, uint16 numSessions) internal returns (TssKey memory) {
        return TestUtils.tssKey(new Signer(privateKey), numSessions);
    }

    function tssKey(Signer signer, uint16 numSessions) internal view returns (TssKey memory) {
        return TssKey({xCoord: signer.xCoord(), yParity: signer.yParity(), numSessions: numSessions});
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

    function makeBatch(uint64 batch) internal pure returns (Batch memory) {
        return TestUtils.makeBatch(batch, new GatewayOp[](0));
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

    function emptyBatch(uint64 batchId) internal pure returns (Batch memory) {
        return TestUtils.makeBatch(batchId);
    }

    function registerBatch(uint64 batchId) internal returns (Batch memory) {
        return TestUtils.makeBatch(batchId, TestUtils.registerOp(TestUtils.tssKey(newShard, 1)));
    }

    function unregisterBatch(uint64 batchId) internal returns (Batch memory) {
        return TestUtils.makeBatch(batchId, TestUtils.unregisterOp(TestUtils.tssKey(shard1, 1)));
    }

    function gmpBatch(uint256 messageSize) internal returns (Batch memory) {
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
        return TestUtils.makeBatch(uint64(messageSize), gmp);
    }

    function measureGas(Gateway gateway, Batch memory batch) internal returns (Gas memory) {
        Signature memory sig = TestUtils.sign(shard2, gateway, batch);

        gateway.execute(sig, batch);
        uint256 gasUsed = vm.lastCallGas().gasTotalUsed;

        uint64 gasLimit = 0;
        uint256 numMsg = 0;
        uint256 msgLen = 0;
        uint256 numReg = 0;
        uint256 numUnreg = 0;
        for (uint256 i = 0; i < batch.ops.length; i++) {
            GatewayOp memory op = batch.ops[i];
            if (op.command == Command.GMP) {
                GmpMessage memory gmp = abi.decode(op.params, (GmpMessage));
                require(uint256(gateway.messages(gmp.messageId())) == uint256(GmpStatus.SUCCESS), "message failed");
                numMsg += 1;
                gasLimit += gmp.gasLimit;
                msgLen += gmp.data.length.align32();
            } else if (op.command == Command.RegisterShard) {
                numReg += 1;
            } else if (op.command == Command.UnregisterShard) {
                numUnreg += 1;
            }
        }

        gateway.execute(sig, batch);
        uint256 sessionGas = vm.lastCallGas().gasTotalUsed;

        bytes memory call = abi.encodeCall(gateway.execute, (sig, batch));

        return Gas({
            numMsg: numMsg,
            numReg: numReg,
            numUnreg: numUnreg,
            msgLen: msgLen,
            calldataLen: call.length,
            sessionGas: sessionGas,
            executionGas: gasUsed - gasLimit - sessionGas
        });
    }
}
