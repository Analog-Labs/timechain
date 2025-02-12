// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/GmpTestTools.t.sol)

pragma solidity >=0.8.0;

import {Test} from "forge-std/Test.sol";
import {MockERC20} from "./MockERC20.sol";
import {TestUtils} from "./TestUtils.sol";
import {GmpTestTools} from "./GmpTestTools.sol";
import {Gateway, GatewayEIP712} from "../src/Gateway.sol";
import {GatewayProxy} from "../src/GatewayProxy.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";
import {IExecutor} from "../src/interfaces/IExecutor.sol";
import {GmpMessage, GmpStatus, GmpSender, PrimitiveUtils} from "../src/Primitives.sol";

contract GmpTestToolsTest is Test {
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for GmpSender;
    using PrimitiveUtils for address;

    address private constant ALICE = address(bytes20(keccak256("Alice")));
    address private constant BOB = address(bytes20(keccak256("Bob")));

    Gateway private constant SEPOLIA_GATEWAY = Gateway(GmpTestTools.SEPOLIA_GATEWAY);
    uint16 private constant SEPOLIA_NETWORK = GmpTestTools.SEPOLIA_NETWORK_ID;

    Gateway private constant SHIBUYA_GATEWAY = Gateway(GmpTestTools.SHIBUYA_GATEWAY);
    uint16 private constant SHIBUYA_NETWORK = GmpTestTools.SHIBUYA_NETWORK_ID;

    /// @dev Test the teleport of tokens from Alice's account in Shibuya to Bob's account in Sepolia
    function testTeleportAliceTokens() external {
        ////////////////////////////////////
        // Step 1: Setup test environment //
        ////////////////////////////////////

        if (msg.data.length > 0) {
            return;
        }

        // Deploy the gateway contracts at pre-defined addresses
        // Also creates one fork for each supported network
        GmpTestTools.setup();

        // Add funds to Alice and Bob in all networks
        GmpTestTools.deal(ALICE, 100 ether);
        GmpTestTools.deal(BOB, 100 ether);

        ///////////////////////////////////////////////////////
        // Step 2: Deploy the sender and recipient contracts //
        ///////////////////////////////////////////////////////

        // Pre-compute the contract addresses, because the contracts must know each other addresses.
        MockERC20 shibuyaErc20 = MockERC20(vm.computeCreateAddress(ALICE, vm.getNonce(ALICE)));
        MockERC20 sepoliaErc20 = MockERC20(vm.computeCreateAddress(BOB, vm.getNonce(BOB)));

        // Switch to Shibuya network and deploy the ERC20 using Alice account
        GmpTestTools.switchNetwork(SHIBUYA_NETWORK, ALICE);
        shibuyaErc20 = new MockERC20("Shibuya ", "A", SHIBUYA_GATEWAY, sepoliaErc20, SEPOLIA_NETWORK, ALICE, 1000);
        assertEq(shibuyaErc20.balanceOf(ALICE), 1000, "unexpected alice balance in shibuya");
        assertEq(shibuyaErc20.balanceOf(BOB), 0, "unexpected bob balance in shibuya");

        // Switch to Sepolia network and deploy the ERC20 using Bob account
        GmpTestTools.switchNetwork(SEPOLIA_NETWORK, BOB);
        sepoliaErc20 = new MockERC20("Sepolia", "B", SEPOLIA_GATEWAY, shibuyaErc20, SHIBUYA_NETWORK, BOB, 0);
        assertEq(sepoliaErc20.balanceOf(ALICE), 0, "unexpected alice balance in sepolia");
        assertEq(sepoliaErc20.balanceOf(BOB), 0, "unexpected bob balance in sepolia");

        // Check if the computed addresses matches
        assertEq(address(shibuyaErc20), vm.computeCreateAddress(ALICE, 0), "unexpected shibuyaErc20 address");
        assertEq(address(sepoliaErc20), vm.computeCreateAddress(BOB, 0), "unexpected sepoliaErc20 address");

        ///////////////////////////////////////////////////////////
        // Step 3: Deposit funds to destination Gateway Contract //
        ///////////////////////////////////////////////////////////

        // Switch to Sepolia network and Alice account
        GmpTestTools.switchNetwork(SEPOLIA_NETWORK, ALICE);

        //////////////////////////////
        // Step 4: Send GMP message //
        //////////////////////////////

        // Switch to Shibuya network and Alice account
        GmpTestTools.switchNetwork(SHIBUYA_NETWORK, ALICE);

        // Teleport 100 tokens from Alice to to Bob's account in sepolia
        // Obs: The `teleport` method internally calls `gateway.submitMessage(...)`
        bytes32 messageID;
        {
            // Estimate the cost of teleporting 100 tokens
            uint256 gmpCost = shibuyaErc20.teleportCost();
            messageID = shibuyaErc20.teleport{value: gmpCost}(BOB, 100);
        }

        // Now with the `messageID`, Alice can check the message status in the destination gateway contract
        // status 0: means the message is pending
        // status 1: means the message was executed successfully
        // status 2: means the message was executed but reverted
        GmpTestTools.switchNetwork(SEPOLIA_NETWORK, ALICE);
        assertTrue(
            SEPOLIA_GATEWAY.gmpInfo(messageID).status == GmpStatus.NOT_FOUND,
            "unexpected message status, expect 'pending'"
        );

        ///////////////////////////////////////////////////
        // Step 5: Wait Chronicles Relay the GMP message //
        ///////////////////////////////////////////////////

        // The GMP hasn't been executed yet...
        assertEq(sepoliaErc20.balanceOf(ALICE), 0, "unexpected alice balance in shibuya");

        // Note: In a live network, the GMP message will be relayed by Chronicle Nodes after a minimum number of confirmations.
        // here we can simulate this behavior by calling `GmpTestTools.relayMessages()`, this will relay all pending messages.
        GmpTestTools.relayMessages();

        // Success! The GMP message was executed!!!
        assertTrue(SEPOLIA_GATEWAY.gmpInfo(messageID).status == GmpStatus.SUCCESS, "failed to execute GMP");

        // Check ALICE and BOB balance in shibuya
        GmpTestTools.switchNetwork(SHIBUYA_NETWORK);
        assertEq(shibuyaErc20.balanceOf(ALICE), 900, "unexpected alice's balance in shibuya");
        assertEq(shibuyaErc20.balanceOf(BOB), 0, "unexpected bob's balance in shibuya");

        // Check ALICE and BOB balance in sepolia
        GmpTestTools.switchNetwork(SEPOLIA_NETWORK);
        assertEq(sepoliaErc20.balanceOf(ALICE), 0, "unexpected alice's balance in sepolia");
        assertEq(sepoliaErc20.balanceOf(BOB), 100, "unexpected bob's balance in sepolia");
    }
}
