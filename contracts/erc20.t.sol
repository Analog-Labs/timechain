// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./erc20.sol";
import "forge-std/Test.sol";

contract ERC20Test is Test {
    TestToken token;
    address constant alice = address(0xa);
    address constant bob = address(0xb);

    function setUp() public {
        token = new TestToken();
        assertEq(token.balanceOf(alice), 0);
        assertEq(token.balanceOf(bob), 0);
    }

    function test_totalSupply() public {
        assertEq(token.totalSupply(), 0);
        token.mint(100);
        assertEq(token.totalSupply(), 100);
        token.burn(100);
        assertEq(token.totalSupply(), 0);
    }

    function test_balanceOf() public {
        vm.startPrank(alice);
        token.mint(100);
        assertEq(token.balanceOf(alice), 100);
        assertEq(token.balanceOf(bob), 0);

        token.transfer(bob, 100);
        assertEq(token.balanceOf(alice), 0);
        assertEq(token.balanceOf(bob), 100);
    }

    function testFail_noBalance() public {
        vm.startPrank(alice);
        token.mint(100);
        assertEq(token.balanceOf(alice), 100);
        assertEq(token.balanceOf(bob), 0);

        token.transfer(bob, 101);
    }

    function test_allowance() public {
        vm.startPrank(alice);
        token.mint(100);
        token.approve(bob, 50);

        vm.startPrank(bob);
        token.transferFrom(alice, bob, 50);

        assertEq(token.balanceOf(alice), 50);
        assertEq(token.balanceOf(bob), 50);
    }

    function testFail_noAllowance() public {
        vm.startPrank(alice);
        token.mint(100);
        token.approve(bob, 50);

        vm.startPrank(bob);
        token.transferFrom(alice, bob, 51);
    }
}
