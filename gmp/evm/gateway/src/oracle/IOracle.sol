// SPDX-License-Identifier: MIT

pragma solidity >=0.8.0;

interface IOracle {
    function getAmountIn(address tokenIn, uint256 amountOut) external view returns (uint256);
    function getGasPrice(uint64 chainId, uint16 ty, uint48 maxAge) external view returns (uint256);
}
