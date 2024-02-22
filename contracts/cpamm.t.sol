// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./cpamm.sol";
import "./erc20.sol";
import "forge-std/Test.sol";

contract CPAMMTest is Test {
    TestToken token0;
    TestToken token1;
    CPAMM amm;

    address constant alice = address(0xa);
    address constant bob = address(0xb);

    function setUp() public {
        token0 = new TestToken();
        token1 = new TestToken();
        amm = new CPAMM(address(token0), address(token1));
    }

    function testFuzz_sqrt(uint x) public {
        unchecked {
            vm.assume(x == 0 || x * x / x == x);
        }
        assertEq(amm._sqrt(x * x), x);
    }

    function test_liquidity() public {
        vm.startPrank(alice);
        token0.mint(100);
        token1.mint(10);
        token0.approve(address(amm), 100);
        token1.approve(address(amm), 10);

        uint shares0 = amm.addLiquidity(100, 10);
        assertEq(shares0, 31); // sqrt(100 * 10)

        vm.startPrank(bob);
        token0.mint(100);
        token1.mint(10);
        token0.approve(address(amm), 100);
        token1.approve(address(amm), 10);

        uint shares1 = amm.addLiquidity(100, 10);
        assertEq(shares1, 31);

        (uint amount0_0, uint amount1_0) = amm.removeLiquidity(shares1);
        assertEq(amount0_0, 100);
        assertEq(amount1_0, 10);

        vm.startPrank(alice);
        (uint amount0_1, uint amount1_1) = amm.removeLiquidity(shares0);
        assertEq(amount0_1, 100);
        assertEq(amount1_1, 10);
    }

    function test_swap() public {
        uint rate = 10;
        uint supply0 = 10000000000000000;
        uint supply1 = supply0 * rate;
        token0.mint(supply0);
        token1.mint(supply1);
        token0.approve(address(amm), supply0);
        token1.approve(address(amm), supply1);
        amm.addLiquidity(supply0, supply1);

        vm.startPrank(alice);
        token0.mint(10000);
        token0.approve(address(amm), 10000);
        uint amount = amm.swap(address(token0), 10000);
        assertEq(amount, 99699);
        assertEq(token0.balanceOf(alice), 0);
        assertEq(token1.balanceOf(alice), amount);
    }
}
