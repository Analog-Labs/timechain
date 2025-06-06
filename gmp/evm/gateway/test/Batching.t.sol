// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Batch.t.sol)

pragma solidity >=0.8.0;

import {Test, console, Vm} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {Signer} from "frost-evm/Signer.sol";
import {TestUtils} from "./TestUtils.sol";
import {GasSpender} from "./GasSpender.sol";
import {Gateway} from "../src/Gateway.sol";
import {GasUtils} from "../src/GasUtils.sol";
import {IGmpReceiver} from "gmp/IGmpReceiver.sol";
import {
    Batch,
    GatewayOp,
    Command,
    GmpMessage,
    GmpCallback,
    Signature,
    TssKey,
    GmpStatus,
    PrimitiveUtils,
    GMP_VERSION
} from "../src/Primitives.sol";

contract Batching is Test {
    using PrimitiveUtils for address;

    uint16 private constant SRC_NETWORK_ID = 1234;
    uint16 private constant DEST_NETWORK_ID = 1337;
    Gateway internal gateway;
    IGmpReceiver internal receiver;

    constructor() {
        gateway = TestUtils.setupGateway(DEST_NETWORK_ID);
        receiver = IGmpReceiver(new GasSpender());
    }

    function test_execute_batch() external {
        uint64 gasLimit = 7845;
        Batch memory batch = TestUtils.makeBatch(
            0,
            GmpMessage({
                source: bytes32(uint256(0xdead_beef)),
                srcNetwork: SRC_NETWORK_ID,
                dest: address(receiver),
                destNetwork: DEST_NETWORK_ID,
                gasLimit: gasLimit,
                nonce: 0,
                data: abi.encode(gasLimit)
            })
        );
        Signature memory sig = TestUtils.sign(TestUtils.shard1, gateway, batch);
        gateway.execute(sig, batch);
    }

    function test_execute_batch_2() external {
        uint64 gasLimit = 7845;
        GatewayOp[] memory ops = new GatewayOp[](2);
        ops[0] = TestUtils.msgOp(
            GmpMessage({
                source: bytes32(uint256(0xdead_beef)),
                srcNetwork: SRC_NETWORK_ID,
                dest: address(receiver),
                destNetwork: DEST_NETWORK_ID,
                gasLimit: gasLimit,
                nonce: 0,
                data: abi.encode(gasLimit)
            })
        );
        ops[1] = TestUtils.msgOp(
            GmpMessage({
                source: bytes32(uint256(0xdead_beef)),
                srcNetwork: SRC_NETWORK_ID,
                dest: address(receiver),
                destNetwork: DEST_NETWORK_ID,
                gasLimit: gasLimit,
                nonce: 1,
                data: abi.encode(gasLimit)
            })
        );
        Batch memory batch = TestUtils.makeBatch(0, ops);
        Signature memory sig = TestUtils.sign(TestUtils.shard1, gateway, batch);
        gateway.execute(sig, batch);
    }
}
