// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Batch.t.sol)

pragma solidity >=0.8.0;

import {Test, console, Vm} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {FactoryUtils} from "../lib/universal-factory/src/FactoryUtils.sol";
import {IUniversalFactory} from "../lib/universal-factory/src/IUniversalFactory.sol";
import {TestUtils, SigningKey, SigningUtils} from "./TestUtils.sol";
import {GasSpender} from "./utils/GasSpender.sol";
import {BaseTest} from "./utils/BaseTest.sol";
import {Gateway, GatewayEIP712} from "../src/Gateway.sol";
import {GatewayProxy} from "../src/GatewayProxy.sol";
import {Hashing} from "../src/utils/Hashing.sol";
import {GasUtils} from "../src/utils/GasUtils.sol";
import {BranchlessMath} from "../src/utils/BranchlessMath.sol";
import {UFloat9x56, UFloatMath} from "../src/utils/Float9x56.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";
import {IGmpReceiver} from "../src/interfaces/IGmpReceiver.sol";
import {IExecutor} from "../src/interfaces/IExecutor.sol";
import {
    InboundMessage,
    GatewayOp,
    Command,
    GmpMessage,
    GmpCallback,
    UpdateKeysMessage,
    Signature,
    TssKey,
    Network,
    GmpStatus,
    PrimitiveUtils,
    GmpSender,
    GMP_VERSION
} from "../src/Primitives.sol";

contract Batching is BaseTest {
    using PrimitiveUtils for UpdateKeysMessage;
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for GmpSender;
    using PrimitiveUtils for address;
    using BranchlessMath for uint256;
    using SigningUtils for SigningKey;
    using FactoryUtils for IUniversalFactory;

    // Chronicle TSS Secret
    uint256 private constant SECRET = 0x42;
    uint256 private constant SIGNING_NONCE = 0x69;

    // Netowrk ids
    uint16 private constant SRC_NETWORK_ID = 1234;
    uint16 internal constant DEST_NETWORK_ID = 1337;

    address private constant RECEIVER_CONTRACT = 0x9888d4d78827bE0C2aDeb578019D35b5a36E80a4;
    address private constant RECEIVER_CONTRACT_02 = address(
        uint160(
            uint256(
                keccak256(
                    abi.encodePacked(
                        uint8(0xff), address(FACTORY), uint256(1), keccak256(type(GasSpender).creationCode)
                    )
                )
            )
        )
    );

    // Domain Separators
    bytes32 private constant DOMAIN_TYPED_HASH =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");

    bytes32 private constant SRC_DOMAIN_SEPARATOR = keccak256(
        abi.encode(
            DOMAIN_TYPED_HASH,
            keccak256("Analog Gateway Contract"),
            keccak256("0.1.0"),
            uint256(SRC_NETWORK_ID),
            GATEWAY_PROXY
        )
    );
    bytes32 private DST_DOMAIN_SEPARATOR = keccak256(
        abi.encode(
            DOMAIN_TYPED_HASH,
            keccak256("Analog Gateway Contract"),
            keccak256("0.1.0"),
            uint256(DEST_NETWORK_ID),
            GATEWAY_PROXY
        )
    );

    address private constant DEPLOYER = 0xbe862AD9AbFe6f22BCb087716c7D89a26051f74C;
    uint256 private constant ADMIN_SECRET = 0x955acb49dbb669143455ffbf98e30ae5b2d95343c8b46ce10bf1975d722e8001;
    address private constant ADMIN = 0xBf3C099fAAC29F7AF0d883374A790f9b3B06c93A;

    // Gateway Proxy
    bytes private constant PROXY_BYTECODE = abi.encodePacked(type(GatewayProxy).creationCode, abi.encode(ADMIN));
    bytes32 private constant PROXY_BYTECODE_HASH = keccak256(PROXY_BYTECODE);
    address payable private constant GATEWAY_PROXY = payable(
        address(
            uint160(
                uint256(
                    keccak256(
                        abi.encodePacked(
                            uint8(0xff),
                            address(FACTORY),
                            uint256(0),
                            keccak256(abi.encodePacked(type(GatewayProxy).creationCode, abi.encode(ADMIN)))
                        )
                    )
                )
            )
        )
    );

    // Gateway Implementation
    bytes private constant IMPLEMENTATION_BYTECODE =
        abi.encodePacked(type(Gateway).creationCode, abi.encode(DEST_NETWORK_ID, GATEWAY_PROXY));
    bytes32 private constant IMPLEMENTATION_BYTECODE_HASH = keccak256(IMPLEMENTATION_BYTECODE);
    address private constant GATEWAY_IMPLEMENTATION = address(
        uint160(
            uint256(
                keccak256(abi.encodePacked(uint8(0xff), address(FACTORY), uint256(0), IMPLEMENTATION_BYTECODE_HASH))
            )
        )
    );

    bytes32 private constant SIGNING_HASH = keccak256(abi.encode(GATEWAY_PROXY, GATEWAY_IMPLEMENTATION));

    constructor() {
        // Create the Admin account
        SigningKey memory signer = TestUtils.createSigner(ADMIN_SECRET);
        console.logBytes(abi.encodePacked(signer.pubkey.px, signer.pubkey.py));
        assertEq(ADMIN, signer.addr(), "admin address missmatch");

        // Deploy `GasSpender` contract
        vm.prank(ADMIN, ADMIN);
        assertEq(
            RECEIVER_CONTRACT,
            FACTORY.create3(bytes32(0), type(GasSpender).creationCode),
            "GasSpender address missmatch"
        );

        assertEq(
            RECEIVER_CONTRACT_02,
            FACTORY.create2(bytes32(uint256(1)), type(GasSpender).creationCode),
            "GasSpender address missmatch"
        );

        // Deposit funds to the ADMIN account
        vm.deal(ADMIN, 100 ether);
    }

    function setUp() external {
        console.log("-- STEP 01");
        {
            // Encode the `IGmpReceiver.onGmpReceived` call
            uint256 gasToWaste = 1000;
            bytes memory encodedCall = abi.encodeCall(
                IGmpReceiver.onGmpReceived,
                (
                    0x0000000000000000000000000000000000000000000000000000000000000000,
                    1,
                    0x0000000000000000000000000000000000000000000000000000000000000000,
                    0,
                    abi.encode(gasToWaste)
                )
            );
            uint256 gasLimit = TestUtils.calculateBaseCost(encodedCall) + gasToWaste;
            TestUtils.executeCall(ADMIN, address(RECEIVER_CONTRACT), gasLimit, 0, encodedCall);
        }
        console.log("-- STEP 02");

        SigningKey memory signer = TestUtils.createSigner(SECRET);
        vm.deal(DEPLOYER, 100 ether);
        vm.startPrank(DEPLOYER, DEPLOYER);
        assertEq(
            GATEWAY_PROXY,
            FACTORY.computeCreate2Address(bytes32(0), PROXY_BYTECODE_HASH),
            "GatewayProxy address missmatch"
        );
        // bytes memory bytecode = abi.encodePacked(type(Gateway).creationCode, abi.encode(DEST_NETWORK_ID, GATEWAY_PROXY));
        console.log("implementation:", GATEWAY_IMPLEMENTATION);
        console.log("         proxy:", GATEWAY_PROXY);
        assertEq(
            GATEWAY_IMPLEMENTATION,
            FACTORY.create2(bytes32(0), IMPLEMENTATION_BYTECODE),
            "Gateway implementation address missmatch"
        );

        // 2 - Deploy the Proxy Contract
        console.log("-- Deploying GatewayProxy");
        TssKey[] memory keys = new TssKey[](1);
        // keys[0] = TssKey({yParity: signer.yParity() == 28 ? 1 : 0, xCoord: signer.xCoord()}); // Shard key
        keys[0] = TssKey({yParity: signer.yParity() == 28 ? 3 : 2, xCoord: signer.xCoord()}); // Shard key
        Network[] memory networks = new Network[](2);
        networks[0].id = SRC_NETWORK_ID; // sepolia network id
        networks[0].gateway = GATEWAY_PROXY; // sepolia proxy address
        networks[1].id = DEST_NETWORK_ID; // shibuya network id
        networks[1].gateway = GATEWAY_PROXY; // shibuya proxy address
        bytes memory initializer = abi.encodeCall(Gateway.initialize, (ADMIN, keys, networks));
        // console.logBytes(initializer);

        // vm.startStateDiffRecording();
        {
            VmSafe.Wallet memory adminSigner = vm.createWallet(ADMIN_SECRET);
            (uint8 v, bytes32 r, bytes32 s) = vm.sign(adminSigner, SIGNING_HASH);
            bytes memory authorization = abi.encode(v, r, s, GATEWAY_IMPLEMENTATION);
            assertEq(
                GATEWAY_PROXY,
                FACTORY.create2(bytes32(0), PROXY_BYTECODE, authorization, initializer),
                "GatewayProxy address missmatch"
            );
        }

        // Deposit funds to the gateway contract
        Gateway(GATEWAY_PROXY).deposit{value: 10 ether}();
    }

    function sign(SigningKey memory signer, GmpMessage memory gmp) private pure returns (Signature memory) {
        GmpCallback memory callback = gmp.memToCallback();
        (uint256 e, uint256 s) = signer.signPrehashed(callback.eip712hash, SIGNING_NONCE);
        return Signature({xCoord: signer.xCoord(), e: e, s: s});
    }

    function computeGmpMessageID(GmpMessage calldata message) external pure returns (bytes32) {
        console.logBytes(msg.data);
        return message.intoCallback().eip712hash;
    }

    function sign(SigningKey memory signer, InboundMessage memory message)
        private
        view
        returns (Signature memory sig)
    {
        signAt(signer, message, sig);
    }

    function computeInboundMessageSigningHash(InboundMessage calldata message) external pure returns (bytes32) {
        bytes32 rootHash = bytes32(0);

        GatewayOp[] calldata ops = message.ops;
        for (uint256 i = 0; i < ops.length; i++) {
            GatewayOp calldata op = ops[i];
            bytes calldata params = op.params;

            bytes32 operationHash;
            if (op.command == Command.GMP) {
                GmpMessage calldata gmp;
                assembly {
                    gmp := add(params.offset, 0x20)
                }
                operationHash = gmp.intoCallback().eip712hash;
            } else {
                TssKey calldata tssKey;
                assembly {
                    tssKey := params.offset
                }
                operationHash = Hashing.hash(tssKey.yParity, tssKey.xCoord);
            }
            rootHash = Hashing.hash(uint256(rootHash), uint256(op.command), uint256(operationHash));
        }
        rootHash = Hashing.hash(message.version, message.batchID, uint256(rootHash));
        return keccak256(
            abi.encodePacked(
                "Analog GMP v2", DEST_NETWORK_ID, bytes32(uint256(uint160(address(GATEWAY_PROXY)))), rootHash
            )
        );
    }

    function signAt(SigningKey memory signer, InboundMessage memory message, Signature memory sig) private view {
        bytes32 signingHash = this.computeInboundMessageSigningHash(message);
        (uint256 e, uint256 s) = signer.signPrehashed(signingHash, SIGNING_NONCE);
        sig.xCoord = signer.xCoord();
        sig.e = e;
        sig.s = s;
    }

    function test_gmp_debug() external {
        vm.txGasPrice(1);
        SigningKey memory signer = TestUtils.createSigner(SECRET);

        // Build and sign GMP message
        // bytes memory data = new bytes(3070 + 32);
        GmpMessage memory gmp = GmpMessage({
            source: ADMIN.toSender(false),
            srcNetwork: SRC_NETWORK_ID,
            dest: RECEIVER_CONTRACT,
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 7845,
            nonce: 0,
            data: new bytes(800)
        });
        {
            bytes memory data = gmp.data;
            assembly {
                mstore(add(data, 0x20), 7845)
            }
        }

        Signature memory sig = sign(signer, gmp);

        vm.deal(DEPLOYER, 100 ether);
        vm.startPrank(DEPLOYER, DEPLOYER);
        uint256 gasNeeded = GasUtils.executionGasNeeded(uint16(gmp.data.length), gmp.gasLimit);
        uint256 executionCost = GasUtils._executionGasCost(uint16(gmp.data.length), gmp.gasLimit);
        emit log_named_uint("    gas needed", gasNeeded);

        require(GATEWAY_PROXY.code.length > 0, "gateway proxy not found");
        Gateway(GATEWAY_PROXY).execute{gas: gasNeeded}(sig, gmp);

        emit log_named_uint("    total cost", GasUtils.computeExecutionRefund(uint16(gmp.data.length), gmp.gasLimit));
        emit log_named_uint("execution cost", executionCost);
    }

    function batchExecute(Signature calldata, InboundMessage calldata message) external pure {
        console.log("batchExecute:");
        console.logBytes(msg.data);
        for (uint256 i = 0; i < message.ops.length; i++) {
            console.log("\nop:", i);
            GatewayOp calldata op = message.ops[i];
            bytes calldata data = op.params;
            console.logBytes(data);

            uint256 offset;
            GmpMessage calldata gmp;
            assembly {
                offset := data.offset
                gmp := add(offset, 0x20)
            }
            data = gmp.data;

            bytes32 value;
            assembly {
                offset := data.offset
                value := calldataload(value)
            }

            console.log(offset, vm.toString(value));
            console.logBytes(data);
        }
    }

    function test_buildBatch() external view {
        GatewayOp[] memory ops = new GatewayOp[](2);
        ops[0] = GatewayOp({
            command: Command.GMP,
            params: abi.encode(
                GmpMessage({
                    source: GmpSender.wrap(0x7777777777777777777777777777777777777777777777777777777777777777),
                    srcNetwork: 0x8888,
                    dest: 0x9999999999999999999999999999999999999999,
                    destNetwork: 0xAAAA,
                    gasLimit: 0xBBBBBBBBBBBBBBBB,
                    nonce: 0xCCCCCCCCCCCCCCCC,
                    data: abi.encode(uint256(0xDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD))
                })
            )
        });
        // data: hex"DDDDDDDDDDDDDDDDDDDDDDDDDDDDDD"

        ops[1] = GatewayOp({
            command: Command.GMP,
            params: abi.encode(
                GmpMessage({
                    source: GmpSender.wrap(0x7070707070707070707070707070707070707070707070707070707070707070),
                    srcNetwork: 0x8080,
                    dest: 0x9090909090909090909090909090909090909090,
                    destNetwork: 0xA0A0,
                    gasLimit: 0xB0B0B0B0B0B0B0B0,
                    nonce: 0xC0C0C0C0C0C0C0C0,
                    data: abi.encode(uint256(0xD0D0D0D0D0D0D0D0D0D0D0D0D0D0D0))
                })
            )
        });
        // data: hex"D0D0D0D0D0D0D0D0D0D0D0D0D0D0D0"

        bytes memory b = abi.encodeCall(
            Gateway.batchExecute,
            (
                Signature({
                    xCoord: 0x1111111111111111111111111111111111111111111111111111111111111111,
                    e: 0x2222222222222222222222222222222222222222222222222222222222222222,
                    s: 0x3333333333333333333333333333333333333333333333333333333333333333
                }),
                InboundMessage({version: 0x44, batchID: 0x5555555555555555, ops: ops})
            )
        );
        console.logBytes(b);
        (bool success,) = address(this).staticcall(b);
        require(success, "call failed");
        // assertEq(b.length, 1234);
    }

    function test_buildBatch2() external view {
        SigningKey memory signer = TestUtils.createSigner(SECRET);

        // Build and sign GMP
        uint64 gasLimit = 7845;
        GatewayOp[] memory ops = new GatewayOp[](1);
        ops[0] = GatewayOp({
            command: Command.GMP,
            params: abi.encode(
                GmpMessage({
                    source: DEPLOYER.toSender(false),
                    srcNetwork: SRC_NETWORK_ID,
                    dest: RECEIVER_CONTRACT,
                    destNetwork: DEST_NETWORK_ID,
                    gasLimit: gasLimit,
                    nonce: 0,
                    data: abi.encode(gasLimit)
                })
            )
        });
        Signature memory sig = Signature({xCoord: 0, e: 0, s: 0});
        InboundMessage memory inbound =
            InboundMessage({version: 1, batchID: uint64(uint256(keccak256("some batch"))), ops: ops});

        // console.log("will sign...");
        signAt(signer, inbound, sig);
        bytes memory b = abi.encodeCall(Gateway.batchExecute, (sig, inbound));
        console.logBytes(b);
        (bool success,) = address(this).staticcall(b);
        require(success, "call failed");
        // assertEq(b.length, 1234);
    }

    function test_batch_debug() external {
        vm.txGasPrice(1);
        SigningKey memory signer = TestUtils.createSigner(SECRET);
        vm.deal(signer.addr(), 100 ether);
        uint64 gasLimit = 7845;

        /////////////////////
        // Build the batch //
        /////////////////////
        GatewayOp[] memory ops = new GatewayOp[](2);
        ops[0] = GatewayOp({
            command: Command.GMP,
            params: abi.encode(
                GmpMessage({
                    source: DEPLOYER.toSender(false),
                    srcNetwork: SRC_NETWORK_ID,
                    dest: RECEIVER_CONTRACT,
                    destNetwork: DEST_NETWORK_ID,
                    gasLimit: gasLimit,
                    nonce: 0,
                    data: abi.encode(gasLimit)
                })
            )
        });
        ops[1] = GatewayOp({
            command: Command.GMP,
            params: abi.encode(
                GmpMessage({
                    source: DEPLOYER.toSender(false),
                    srcNetwork: SRC_NETWORK_ID,
                    dest: RECEIVER_CONTRACT_02,
                    destNetwork: DEST_NETWORK_ID,
                    gasLimit: gasLimit,
                    nonce: 0,
                    data: abi.encode(gasLimit)
                })
            )
        });
        InboundMessage memory inbound =
            InboundMessage({version: 1, batchID: uint64(uint256(keccak256("some batch"))), ops: ops});

        ////////////////////
        // Sign the batch //
        ////////////////////
        Signature memory sig = sign(signer, inbound);
        bytes memory encodedCall = abi.encodeCall(Gateway.batchExecute, (sig, inbound));

        console.log("encoded call:");
        console.logBytes(encodedCall);

        // vm.deal(DEPLOYER, 100 ether);
        // vm.startPrank(DEPLOYER, DEPLOYER);
        uint256 gasNeeded = GasUtils.executionGasNeeded(uint16(32), gasLimit);
        uint256 executionCost = GasUtils._executionGasCost(uint16(32), gasLimit);
        emit log_named_uint("    gas needed", gasNeeded);
        emit log_named_uint("execution cost", executionCost);
        require(GATEWAY_PROXY.code.length > 0, "gateway proxy not found");

        uint256 baseCost;
        bool success;
        bytes memory result;
        // (executionCost, baseCost, success, result) = address(GATEWAY_PROXY).call(encodedCall);
        console.log("will execute..");
        (executionCost, baseCost, success, result) =
            TestUtils.tryExecuteCall(signer.addr(), GATEWAY_PROXY, 500_000, 0, encodedCall);
        emit log_named_uint("execution cost", executionCost);
        emit log_named_uint("     base cost", baseCost);
        if (!success) {
            console.log("reverted:");
            console.logBytes(result);
            assembly {
                revert(add(result, 0x20), mload(result))
            }
        }

        emit log_named_uint("    total cost", GasUtils.computeExecutionRefund(uint16(32), gasLimit));
        emit log_named_uint("execution cost", executionCost);
    }
}
