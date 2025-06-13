// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/storage/Routes.sol)
pragma solidity ^0.8.20;

import {Signature, Route, MAX_PAYLOAD_SIZE} from "../Primitives.sol";
import {EnumerableMap} from "@openzeppelin/contracts/utils/structs/EnumerableMap.sol";
import {GasUtils} from "../GasUtils.sol";

/**
 * @dev EIP-7201 Route's Storage
 */
library RouteStore {
    using EnumerableMap for EnumerableMap.UintToUintMap;

    /**
     * @dev Namespace of the routes storage `analog.one.gateway.routes`.
     * keccak256(abi.encode(uint256(keccak256("analog.one.gateway.routes")) - 1)) & ~bytes32(uint256(0xff));
     */
    bytes32 private constant _EIP7201_NAMESPACE = 0xb184f2aad520cf7f1f1270909517c75ae33cdf2bd7d32b997a96577f11a48800;

    /**
     * @dev Network info stored in the Gateway Contract
     * @param gasLimit The maximum amount of gas we allow on this particular network.
     * @param relativeGasPriceNumerator Gas price of destination chain, in terms of the source chain token.
     * @param relativeGasPriceDenominator Gas price of destination chain, in terms of the source chain token.
     * @param baseFee Base fee for cross-chain message approval on destination, in terms of source native gas token.
     */
    struct NetworkInfo {
        bytes32 gateway;
        uint64 maxGasLimit;
        uint64 msgGas;
        uint64 msgByteGas;
        uint64 gasPriceMantissa;
        int16 gasPriceExponent;
        uint64 msgFee;
    }

    /**
     * @dev Emitted when a route is updated.
     * @param networkId Network identifier.
     * @param maxGasLimit The maximum amount of gas we allow on this particular network.
     * @param msgGas.
     * @param msgByteGas.
     * @param msgFee Base fee for cross-chain message approval on destination, in terms of source native gas token.
     */
    event RouteUpdated(
        uint16 indexed networkId,
        bytes32 gateway,
        uint64 maxGasLimit,
        uint64 msgGas,
        uint64 msgByteGas,
        uint64 gasPriceMantissa,
        int16 gasPriceExponent,
        uint64 msgFee
    );

    /**
     * @dev Shard info stored in the Gateway Contract
     * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
     * the shard info below 256 bit, so it can be stored in one single storage slot.
     * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
     *
     * @custom:storage-location erc7201:analog.one.gateway.routes
     */
    struct MainStorage {
        EnumerableMap.UintToUintMap routeIds;
        mapping(uint16 => NetworkInfo) routes;
    }

    function getMainStorage() internal pure returns (MainStorage storage $) {
        assembly {
            $.slot := _EIP7201_NAMESPACE
        }
    }

    /**
     * @dev Returns the value associated with `NetworkInfo`. O(1).
     *
     * Requirements:
     * - `NetworkInfo` must be in the map.
     */
    function get(MainStorage storage store, uint16 id) internal view returns (NetworkInfo storage) {
        if (!store.routeIds.contains(uint256(id))) {
            revert("missing route");
        }
        return store.routes[id];
    }

    function insert(MainStorage storage store, Route calldata route) internal {
        uint256 networkId = uint256(route.networkId);
        NetworkInfo storage stored = store.routes[route.networkId];

        if (!store.routeIds.contains(networkId)) {
            store.routeIds.set(networkId, 1);
        }

        stored.gateway = route.gateway;
        stored.maxGasLimit = route.maxGasLimit;
        stored.msgGas = route.msgGas;
        stored.msgByteGas = route.msgByteGas;
        stored.gasPriceMantissa = route.gasPriceMantissa;
        stored.gasPriceExponent = route.gasPriceExponent;
        stored.msgFee = route.msgFee;

        emit RouteUpdated(
            route.networkId,
            stored.gateway,
            stored.maxGasLimit,
            stored.msgGas,
            stored.msgByteGas,
            stored.gasPriceMantissa,
            stored.gasPriceExponent,
            stored.msgFee
        );
    }

    /**
     * @dev Return all routes registered currently registered.
     *
     * WARNING: This operation will copy the entire storage to memory, which can be quite expensive. This is designed
     * to mostly be used by view accessors that are queried without any gas fees. Developers should keep in mind that
     * this function has an unbounded cost, and using it as part of a state-changing function may render the function
     * uncallable if the set grows to a point where copying to memory consumes too much gas to fit in a block.
     */
    function list(MainStorage storage store) internal view returns (Route[] memory) {
        uint256 len = store.routeIds.length();
        Route[] memory routes = new Route[](len);

        for (uint256 i = 0; i < len; i++) {
            (uint256 key,) = store.routeIds.at(i);
            uint16 networkId = uint16(key);
            NetworkInfo storage route = store.routes[networkId];

            routes[i] = Route({
                networkId: networkId,
                gateway: route.gateway,
                maxGasLimit: route.maxGasLimit,
                msgGas: route.msgGas,
                msgByteGas: route.msgByteGas,
                gasPriceMantissa: route.gasPriceMantissa,
                gasPriceExponent: route.gasPriceExponent,
                msgFee: route.msgFee
            });
        }
        return routes;
    }

    function estimateGas(NetworkInfo memory route, uint256 messageSize, uint64 gasLimit)
        internal
        pure
        returns (uint256)
    {
        // Verify if the gas limit and message size are within the limits
        require(gasLimit <= route.maxGasLimit, "gas limit exceeded");
        require(messageSize <= MAX_PAYLOAD_SIZE, "maximum payload size exceeded");
        return messageSize * route.msgByteGas + route.msgGas + gasLimit;
    }

    function estimateCost(NetworkInfo memory route, uint256 gas) internal pure returns (uint256 cost) {
        require(route.gasPriceMantissa != 0, "route is temporarily disabled");
        cost = gas * route.gasPriceMantissa;
        if (route.gasPriceExponent > 0) {
            cost <<= uint16(route.gasPriceExponent);
        } else {
            cost >>= uint16(-route.gasPriceExponent);
        }
        cost += route.msgFee;
    }
}
