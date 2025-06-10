// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/Gateway.sol)

pragma solidity >=0.8.0;

interface IPriceOracle {
    function getPrice(address token) external view returns (uint256, uint256);
    function getAmountIn(address tokenIn, address tokenOut, uint256 amountOut) external view returns (uint256);
}

interface IGasPriceOracle {
    function getGasPrice(uint64 chainId, uint16 ty, uint48 maxAge) external view returns (uint256 value);
}
