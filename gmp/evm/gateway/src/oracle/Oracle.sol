// SPDX-License-Identifier: MIT

pragma solidity >=0.8.0;

import {IOracle} from "./IOracle.sol";

interface IUniswapV2Factory {
    function getPair(address tokenA, address tokenB) external view returns (address pair);
}

interface IUniswapV2Pair {
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    function token0() external view returns (address);
    function token1() external view returns (address);
}

interface IERC20 {
    function decimals() external view returns (uint8);
}

interface IGasNetwork {
    /**
     * @param systemid:
     * 1 for Bitcoin chains
     * 2 for Evm chains
     * @param cid:
     * chainId of the chain
     * @param typ:
     * 107: Base fee (EIP-1559)
     * 115: Blob base fee (post-EIP-4844 chains)
     * 322: 90th percentile priority fee
     * @param tin:
     * miliseconds, return zero if the data is older than mili seconds
     */
    function getInTime(uint8 systemid, uint64 cid, uint16 typ, uint48 tin)
        external
        view
        returns (uint256 value, uint64 height, uint48 timestamp);
}

contract Oracle is IOracle {
    address public immutable uniswapv2Factory;
    address public immutable nativeWrappedErc20;
    address public immutable gasNetwork;

    constructor(address _uniswapv2Factory, address _nativeWrappedErc20, address _gasNetwork) {
        uniswapv2Factory = _uniswapv2Factory;
        gasNetwork = _gasNetwork;
        nativeWrappedErc20 = _nativeWrappedErc20;
    }

    function getGasPrice(uint64 chainId, uint16 ty, uint48 tin) external view returns (uint256 value) {
        (uint256 gasPrice,,) = IGasNetwork(gasNetwork).getInTime(2, chainId, ty, tin);
        require(gasPrice != 0, "Failed to get gas price");
        return gasPrice;
    }

    function getAmountIn(address token, uint256 amountOut) external view returns (uint256) {
        address pairAddress = IUniswapV2Factory(uniswapv2Factory).getPair(token, nativeWrappedErc20);
        require(pairAddress != address(0), "Pair does not exist");

        IUniswapV2Pair pair = IUniswapV2Pair(pairAddress);

        (uint112 reserve0, uint112 reserve1,) = pair.getReserves();

        bool isToken0In = pair.token0() == nativeWrappedErc20;
        (uint256 reserveIn, uint256 reserveOut) =
            isToken0In ? (uint256(reserve0), uint256(reserve1)) : (uint256(reserve1), uint256(reserve0));
        require(reserveOut > amountOut, "Insufficient liquidity");

        uint256 numerator = reserveIn * amountOut * 1000;
        // uniswap v2 fee is 0.3%
        uint256 denominator = (reserveOut - amountOut) * 997;
        uint256 amountIn = (numerator / denominator) + 1;

        return amountIn;
    }
}
