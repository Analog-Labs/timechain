// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Example.t.sol)

pragma solidity >=0.8.0;

import {Test} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {console} from "forge-std/console.sol";
import {Random} from "./Random.sol";
import {MockERC20} from "./MockERC20.sol";
import {GmpTestTools} from "./GmpTestTools.sol";
import {TestUtils, SigningKey, VerifyingKey, SigningUtils, VerifyingUtils} from "./TestUtils.sol";
import {Gateway, GatewayEIP712} from "../src/Gateway.sol";
import {GatewayProxy} from "../src/GatewayProxy.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";
import {IGmpReceiver} from "../src/interfaces/IGmpReceiver.sol";
import {IExecutor} from "../src/interfaces/IExecutor.sol";
import {GasUtils} from "../src/utils/GasUtils.sol";
import {
    GmpMessage,
    UpdateKeysMessage,
    Signature,
    TssKey,
    Network,
    GmpStatus,
    GmpSender,
    PrimitiveUtils
} from "../src/Primitives.sol";

contract ExampleTest is Test {
    using SigningUtils for SigningKey;
    using VerifyingUtils for VerifyingKey;
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for address;

    uint16 private constant SRC_NETWORK_ID = 1234;
    uint16 private constant DEST_NETWORK_ID = 1337;
    uint256 private constant SENDER_SECRET = uint248(uint256(keccak256("secret")));
    address private _sender;

    address private constant ALICE = address(bytes20(keccak256("Alice")));
    address private constant BOB = address(bytes20(keccak256("Bob")));

    function setUp() external {
        vm.deal(ALICE, 100 ether);
        vm.deal(BOB, 100 ether);
    }

    function deployGateway(VmSafe.Wallet memory admin, SigningKey memory signer, uint16[] memory networkIds)
        private
        returns (Network[] memory networks)
    {
        TssKey[] memory keys = new TssKey[](1);
        keys[0] = TssKey({yParity: signer.pubkey.yParity() == 28 ? 3 : 2, xCoord: signer.pubkey.px});

        networks = new Network[](networkIds.length);
        for (uint256 i = 0; i < networks.length; i++) {
            networks[i].id = networkIds[i];
            networks[i].gateway = TestUtils.computeGatewayProxyAddress(admin.addr, bytes32(uint256(networks[i].id)));
            vm.deal(networks[i].gateway, 100 ether);
        }

        // bytes memory initializer = abi.encodeCall(Gateway.initialize, (msg.sender, keys, networks));
        for (uint256 i = 0; i < networks.length; i++) {
            address proxy =
                address(TestUtils.setupGateway(admin, bytes32(uint256(networks[i].id)), networks[i].id, keys, networks));
            assertEq(proxy, networks[i].gateway, "GatewayProxy address mismatch");
            vm.deal(proxy, 100 ether);
        }
    }

    function testSignature() external pure {
        SigningKey memory sk = TestUtils.createSigner();
        VerifyingKey memory vk = sk.pubkey;
        (uint256 c, uint256 z) = sk.sign("hello world!", Random.nextUint());
        assertTrue(vk.verify("hello world!", c, z), "invalid signature");
    }

    function testTeleportTokens() external {
        vm.txGasPrice(1);
        VmSafe.Wallet memory senderWallet = vm.createWallet(SENDER_SECRET);
        _sender = senderWallet.addr;
        vm.deal(_sender, 100 ether);

        // Step 1: Deploy the Gateway contract
        SigningKey memory signer = TestUtils.createSigner();
        Gateway srcGateway;
        Gateway dstGateway;
        {
            uint16[] memory networkIds = new uint16[](2);
            networkIds[0] = SRC_NETWORK_ID;
            networkIds[1] = DEST_NETWORK_ID;
            Network[] memory networks = deployGateway(senderWallet, signer, networkIds);
            srcGateway = Gateway(payable(networks[0].gateway));
            dstGateway = Gateway(payable(networks[1].gateway));
        }

        // Step 2: Deploy the sender and recipient contracts
        vm.startPrank(_sender, _sender);
        MockERC20 srcToken = MockERC20(vm.computeCreateAddress(_sender, vm.getNonce(_sender) + 1));
        MockERC20 dstToken =
            new MockERC20("Destination Token", "B", dstGateway, srcToken, srcGateway.networkId(), ALICE, 0);
        srcToken = new MockERC20("Source Token", "A", srcGateway, dstToken, dstGateway.networkId(), ALICE, 1000);

        // Step 3: Send GMP message
        GmpSender source = address(srcToken).toSender(false);
        GmpMessage memory gmp = GmpMessage({
            source: source,
            srcNetwork: SRC_NETWORK_ID,
            dest: address(dstToken),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 100_000,
            nonce: 0,
            data: abi.encode(MockERC20.CrossChainTransfer({from: ALICE, to: BOB, amount: 100}))
        });

        // Calculate the expect GMP gas cost
        uint256 gasCost;
        {
            uint256 nonZeros = GasUtils.countNonZeros(gmp.data);
            uint256 zeros = gmp.data.length - nonZeros;
            gasCost = GasUtils.estimateGas(uint16(nonZeros), uint16(zeros), gmp.gasLimit);
        }

        // Expect `GmpCreated` to be emitted
        bytes32 messageID = gmp.eip712hash();
        vm.expectEmit(true, true, true, true, address(srcGateway));
        emit IGateway.GmpCreated(
            messageID,
            GmpSender.unwrap(gmp.source),
            gmp.dest,
            gmp.destNetwork,
            gmp.gasLimit,
            uint64(gasCost),
            gmp.nonce,
            gmp.data
        );

        {
            // Estimate the cost of teleporting 100 tokens
            uint256 gmpCost = srcToken.teleportCost();

            // Submit the GMP message from `sender` contract
            vm.stopPrank();
            vm.prank(ALICE, ALICE);
            srcToken.teleport{value: gmpCost}(BOB, 100);
        }

        vm.startPrank(_sender, _sender);
        (uint256 c, uint256 z) = signer.signPrehashed(messageID, Random.nextUint());
        Signature memory sig = Signature({xCoord: signer.pubkey.px, e: c, s: z});
        assertTrue(dstGateway.gmpInfo(messageID).status == GmpStatus.NOT_FOUND, "GMP message already executed");

        // Expect `GmpExecuted` to be emitted
        vm.expectEmit(true, true, true, true, address(dstGateway));
        emit IExecutor.GmpExecuted(messageID, gmp.source, gmp.dest, GmpStatus.SUCCESS, messageID);

        // Execute the GMP message
        dstGateway.execute(sig, gmp);
        assertTrue(dstGateway.gmpInfo(messageID).status == GmpStatus.SUCCESS, "failed to execute GMP message");

        // Check balance
        assertEq(srcToken.balanceOf(ALICE), 900, "sender balance mismatch");
        assertEq(dstToken.balanceOf(ALICE), 0, "recipient balance mismatch");
        assertEq(srcToken.balanceOf(BOB), 0, "sender balance mismatch");
        assertEq(dstToken.balanceOf(BOB), 100, "recipient balance mismatch");
    }
}
