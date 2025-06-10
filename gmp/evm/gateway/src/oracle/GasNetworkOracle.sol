// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/Gateway.sol)

pragma solidity >=0.8.0;

import {IGasPriceOracle} from "src/oracle/IOracle.sol";
import {Test, console} from "forge-std/Test.sol";

interface IGasNetOracle {
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

contract GasNetworkOracle is IGasPriceOracle {
    address public immutable gasNet;

    constructor(address _gasNet) {
        gasNet = _gasNet;
    }

    function getGasPrice(uint64 chainId, uint16 ty, uint48 tin) external view returns (uint256 value) {
        // miliseconds are of two hours need to check on this later.
        (uint256 gasPrice,,) = IGasNetOracle(gasNet).getInTime(2, chainId, ty, tin);
        return gasPrice;
    }
}
