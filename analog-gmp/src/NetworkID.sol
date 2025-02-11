// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/NetworkID.sol)

pragma solidity ^0.8.0;

import {BranchlessMath} from "./utils/BranchlessMath.sol";

type NetworkID is uint16;

library NetworkIDHelpers {
    NetworkID internal constant MAINNET = NetworkID.wrap(0);
    NetworkID internal constant ASTAR = NetworkID.wrap(1);
    NetworkID internal constant POLYGON_POS = NetworkID.wrap(2);
    NetworkID internal constant ETHEREUM_LOCAL_DEV = NetworkID.wrap(3);
    NetworkID internal constant GOERLI = NetworkID.wrap(4);
    NetworkID internal constant SEPOLIA = NetworkID.wrap(5);
    NetworkID internal constant ASTAR_LOCAL_DEV = NetworkID.wrap(6);
    NetworkID internal constant SHIBUYA = NetworkID.wrap(7);
    NetworkID internal constant POLYGON_AMOY = NetworkID.wrap(8);
    NetworkID internal constant BINANCE_SMART_CHAIN_TESTNET = NetworkID.wrap(9);
    NetworkID internal constant ARBITRUM_SEPOLIA = NetworkID.wrap(10);

    /**
     * @dev Converts a `NetworkID` into a `uint16`.
     */
    function asUint(NetworkID networkId) internal pure returns (uint16) {
        return NetworkID.unwrap(networkId);
    }

    /**
     * @dev Get the EIP-150 chain id from the network id.
     */
    function chainId(NetworkID networkId) internal pure returns (uint64 chainID) {
        assembly {
            switch networkId
            case 0 {
                // Ethereum Mainnet
                chainID := 0
            }
            case 1 {
                // Astar
                chainID := 592
            }
            case 2 {
                // Polygon PoS
                chainID := 137
            }
            case 3 {
                // Ethereum local testnet
                chainID := 1337
            }
            case 4 {
                // Goerli
                chainID := 5
            }
            case 5 {
                // Sepolia
                chainID := 11155111
            }
            case 6 {
                // Astar local testnet
                chainID := 592
            }
            case 7 {
                // Shibuya
                chainID := 81
            }
            case 8 {
                // Polygon Amoy
                chainID := 80002
            }
            case 9 {
                // Binance Smart Chain
                chainID := 97
            }
            case 10 {
                // Arbitrum Sepolia
                chainID := 421614
            }
            default {
                // Unknown network id
                chainID := 0xffffffffffffffff
            }
        }
        require(chainID > 2 ** 24, "the provided network id doesn't exists");
        return uint64(chainID);
    }

    /**
     * @dev Try to get the network id from the chain id.
     */
    function tryFromChainID(uint256 chainid) internal pure returns (bool, NetworkID) {
        uint256 networkId = type(uint256).max;

        // Ethereum Mainnet
        networkId = BranchlessMath.ternary(chainid == 0, asUint(MAINNET), networkId);
        // Astar
        networkId = BranchlessMath.ternary(chainid == 592, asUint(ASTAR), networkId);
        // Polygon PoS
        networkId = BranchlessMath.ternary(chainid == 137, asUint(POLYGON_POS), networkId);
        // Ethereum local testnet
        networkId = BranchlessMath.ternary(chainid == 1337, asUint(ETHEREUM_LOCAL_DEV), networkId);
        // Goerli
        networkId = BranchlessMath.ternary(chainid == 5, asUint(GOERLI), networkId);
        // Sepolia
        networkId = BranchlessMath.ternary(chainid == 11155111, asUint(SEPOLIA), networkId);
        // Astar local testnet
        networkId = BranchlessMath.ternary(chainid == 592, asUint(ASTAR_LOCAL_DEV), networkId);
        // Shibuya
        networkId = BranchlessMath.ternary(chainid == 81, asUint(SHIBUYA), networkId);
        // Polygon Amoy
        networkId = BranchlessMath.ternary(chainid == 80002, asUint(POLYGON_AMOY), networkId);
        // Binance Smart Chain
        networkId = BranchlessMath.ternary(chainid == 97, asUint(BINANCE_SMART_CHAIN_TESTNET), networkId);
        // Arbitrum Sepolia
        networkId = BranchlessMath.ternary(chainid == 421614, asUint(ARBITRUM_SEPOLIA), networkId);

        bool exists = networkId != type(uint256).max;
        return (exists, NetworkID.wrap(uint16(networkId)));
    }

    /**
     * @dev Converts a EIP-155 chain id into a `NetworkID`, reverts if the network id doesn't exists.
     */
    function fromChainID(uint256 chainid) internal pure returns (NetworkID) {
        (bool exists, NetworkID networkId) = tryFromChainID(chainid);
        require(exists, "network id doesn't exists for the given chain id");
        return networkId;
    }
}
