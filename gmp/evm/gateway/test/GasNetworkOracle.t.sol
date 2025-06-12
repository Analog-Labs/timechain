// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/Gateway.sol)

pragma solidity >=0.8.0;

import {Test, console} from "forge-std/Test.sol";
import {IOracle} from "src/oracle/IOracle.sol";
import {Oracle} from "src/oracle/Oracle.sol";
import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";

contract GasNetworkOracleTest is Test {
    Oracle oracle;
    address constant ArbitrumMainnet = 0x1c51B22954af03FE11183aaDF43F6415907a9287;
    uint256 constant forkBlock = 346200343;

    function setUp() public {
        vm.createSelectFork("https://arbitrum-one.public.blastapi.io", forkBlock);
        oracle = new Oracle(address(0), address(0), ArbitrumMainnet);
        vm.makePersistent(address(oracle));
    }

    function testGasPrice() public view {
        (uint256 value) = IOracle(oracle).getGasPrice(1, 322, 7200000);
        assert(value > 0);
    }

    function testGetGasPriceInRange() public {
        vm.skip(true);
        uint256 BLOCKS_TO_ITERATE = 50;
        // almost 12k blocks per hour
        uint256 BLOCK_STEP = 100_000;
        string memory path = "gas_price.csv";
        string memory csv = string.concat("chain_id,l2block,l1block,timestamp,gas_price\n");
        string memory rpc = "https://arbitrum-one.public.blastapi.io";
        for (uint256 i = 0; i < BLOCKS_TO_ITERATE; i++) {
            uint256 targetBlock = forkBlock - (i * BLOCK_STEP);
            vm.createSelectFork(rpc, targetBlock);
            uint256 timestamp = block.timestamp;
            // returns l1 block;
            uint256 blck = block.number;
            uint64[2] memory chains = [uint64(1), 42161];
            for (uint256 j = 0; j < chains.length; j++) {
                uint64 cid = chains[j];
                (uint256 value) = IOracle(oracle).getGasPrice(cid, 107, 7200000);
                csv = string.concat(
                    csv,
                    Strings.toString(cid),
                    ",",
                    Strings.toString(targetBlock),
                    ",",
                    Strings.toString(blck),
                    ",",
                    Strings.toString(timestamp),
                    ",",
                    Strings.toString(value),
                    "\n"
                );
            }
        }
        vm.writeFile(path, csv);
    }
}
