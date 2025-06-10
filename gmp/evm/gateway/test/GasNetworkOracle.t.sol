// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/Gateway.sol)

pragma solidity >=0.8.0;

import {Test, console} from "forge-std/Test.sol";
import {IOracle} from "src/oracle/IOracle.sol";
import {Oracle} from "src/oracle/Oracle.sol";

contract GasNetworkOracleTest is Test {
    Oracle oracle;
    address constant ArbitrumMainnet = 0x1c51B22954af03FE11183aaDF43F6415907a9287;

    function setUp() public {
        vm.createSelectFork({urlOrAlias: "https://arb1.arbitrum.io/rpc"});
        oracle = new Oracle(address(0), address(0), ArbitrumMainnet);
    }

    function testGasPrice() public view {
        (uint256 value) = IOracle(oracle).getGasPrice(1, 322, 7200000);
        assert(value > 0);
    }
}
