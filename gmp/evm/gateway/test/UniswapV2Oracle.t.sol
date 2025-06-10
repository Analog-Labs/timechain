// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/Gateway.sol)

pragma solidity >=0.8.0;

import {Test, console} from "forge-std/Test.sol";
import {UniswapV2Oracle} from "src/oracle/UniswapV2Oracle.sol";
import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";

interface IERC20 {
    function decimals() external view returns (uint8);
}

contract UniswapV2OracleTest is Test {
    UniswapV2Oracle oracle;

    address constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    address constant USDT = 0xdAC17F958D2ee523a2206206994597C13D831ec7;
    address constant FACTORY = 0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f;

    function setUp() public {
        vm.createSelectFork({urlOrAlias: "https://eth.meowrpc.com"});
        oracle = new UniswapV2Oracle(FACTORY, USDT);
        vm.makePersistent(address(oracle));
    }

    function testGetNativePrice() public view {
        (uint256 usdPrice,) = oracle.getPrice(WETH);
        console.log("ETH USDT price", usdPrice);
        assert(usdPrice > 0);
    }

    function testGetAmountIn() public view {
        (uint256 usdPrice,) = oracle.getPrice(WETH);
        (uint256 usdtRequired) = oracle.getAmountIn(USDT, WETH, 1 ether);
        uint256 usdtDecimals = IERC20(USDT).decimals();
        uint256 usdtScale = 10 ** usdtDecimals;
        uint256 usdtRequiredForOneEth = usdtRequired / usdtScale;
        console.log("ETH pool price", usdtRequiredForOneEth);
        require(usdtRequiredForOneEth - usdPrice < 50);
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
            (uint256 usdPrice, uint256 fraction) = oracle.getPrice(WETH);
            csv = string.concat(
                csv,
                Strings.toString(targetBlock),
                ",",
                Strings.toString(timestamp),
                ",",
                Strings.toString(usdPrice),
                ".",
                Strings.toString(fraction),
                "\n"
            );
        }

        vm.writeFile(path, csv);
    }
}
