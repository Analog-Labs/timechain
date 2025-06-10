// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/Gateway.sol)

pragma solidity >=0.8.0;

import {Test, console} from "forge-std/Test.sol";
import {Oracle} from "src/oracle/Oracle.sol";
import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";

interface IERC20 {
    function decimals() external view returns (uint8);
}

contract OracleTest is Test {
    Oracle oracle;

    address constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    address constant USDT = 0xdAC17F958D2ee523a2206206994597C13D831ec7;
    address constant FACTORY = 0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f;

    function setUp() public {
        vm.createSelectFork({urlOrAlias: "https://eth.meowrpc.com"});
        oracle = new Oracle(FACTORY, USDT, address(0));
        vm.makePersistent(address(oracle));
    }

    function testGetAmountIn() public view {
        (uint256 usdtRequired) = oracle.getAmountIn(WETH, 1 ether);
        uint256 usdtDecimals = IERC20(USDT).decimals();
        uint256 usdtScale = 10 ** usdtDecimals;
        uint256 usdtRequiredForOneEth = usdtRequired / usdtScale;
        console.log("ETH pool price", usdtRequiredForOneEth);
        require(usdtRequiredForOneEth > 0);
    }

    function testGeneratePricesRange() public {
        // skipping the test due to nature of constant rpc queries
        vm.skip(true);
        uint256 BLOCKS_TO_ITERATE = 50;
        uint256 BLOCK_STEP = 299;
        string memory path = "uni_prices.csv";
        string memory rpc_url = "https://eth-mainnet.public.blastapi.io";

        uint256 startBlock = block.number;
        string memory csv = string.concat("block_number,timestamp,price\n");

        for (uint256 i = 0; i < BLOCKS_TO_ITERATE; i++) {
            uint256 targetBlock = startBlock - (i * BLOCK_STEP);
            vm.createSelectFork(rpc_url, targetBlock);
            uint256 timestamp = block.timestamp;
            uint256 price = oracle.getAmountIn(WETH, 1 ether);
            uint256 tokenDecimals = IERC20(USDT).decimals();
            uint256 scale = 10 ** tokenDecimals;
            uint256 integer_part = price / scale;
            uint256 fraction = price % scale;
            csv = string.concat(
                csv,
                Strings.toString(targetBlock),
                ",",
                Strings.toString(timestamp),
                ",",
                Strings.toString(integer_part),
                ".",
                Strings.toString(fraction),
                "\n"
            );
        }

        vm.writeFile(path, csv);
    }
}
