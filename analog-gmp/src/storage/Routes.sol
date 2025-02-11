// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/storage/Routes.sol)
pragma solidity ^0.8.20;

import {Signature, Network, Route, MAX_PAYLOAD_SIZE} from "../Primitives.sol";
import {NetworkIDHelpers, NetworkID} from "../NetworkID.sol";
import {EnumerableSet, Pointer} from "../utils/EnumerableSet.sol";
import {BranchlessMath} from "../utils/BranchlessMath.sol";
import {UFloat9x56, UFloatMath} from "../utils/Float9x56.sol";
import {StoragePtr} from "../utils/Pointer.sol";
import {GasUtils} from "../utils/GasUtils.sol";

/**
 * @dev EIP-7201 Route's Storage
 */
library RouteStore {
    using Pointer for StoragePtr;
    using Pointer for uint256;
    using EnumerableSet for EnumerableSet.Map;
    using NetworkIDHelpers for NetworkID;
    using UFloatMath for UFloat9x56;
    using BranchlessMath for uint256;

    /**
     * @dev Namespace of the routes storage `analog.one.gateway.routes`.
     * keccak256(abi.encode(uint256(keccak256("analog.one.gateway.routes")) - 1)) & ~bytes32(uint256(0xff));
     */
    bytes32 internal constant _EIP7201_NAMESPACE = 0xb184f2aad520cf7f1f1270909517c75ae33cdf2bd7d32b997a96577f11a48800;

    /**
     * @dev Network info stored in the Gateway Contract
     * @param gasLimit The maximum amount of gas we allow on this particular network.
     * @param relativeGasPrice Gas price of destination chain, in terms of the source chain token.
     * @param baseFee Base fee for cross-chain message approval on destination, in terms of source native gas token.
     */
    struct NetworkInfo {
        uint64 gasLimit;
        UFloat9x56 relativeGasPrice;
        uint128 baseFee;
    }

    /**
     * @dev Emitted when a route is updated.
     * @param networkId Network identifier.
     * @param relativeGasPrice Gas price of destination chain, in terms of the source chain token.
     * @param baseFee Base fee for cross-chain message approval on destination, in terms of source native gas token.
     * @param gasLimit The maximum amount of gas we allow on this particular network.
     */
    event RouteUpdated(uint16 indexed networkId, UFloat9x56 relativeGasPrice, uint128 baseFee, uint64 gasLimit);

    /**
     * @dev Shard info stored in the Gateway Contract
     * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
     * the shard info below 256 bit, so it can be stored in one single storage slot.
     * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
     *
     * @custom:storage-location erc7201:analog.one.gateway.routes
     */
    struct MainStorage {
        EnumerableSet.Map routes;
    }

    error RouteNotExists(NetworkID id);
    error IndexOutOfBounds(uint256 index);

    function getMainStorage() internal pure returns (MainStorage storage $) {
        assembly {
            $.slot := _EIP7201_NAMESPACE
        }
    }

    /**
     * @dev Converts a `StoragePtr` into an `NetworkInfo`.
     */
    function pointerToRoute(StoragePtr ptr) private pure returns (NetworkInfo storage route) {
        assembly {
            route.slot := ptr
        }
    }

    /**
     * @dev Returns true if the value is in the set. O(1).
     */
    function has(MainStorage storage store, NetworkID id) internal view returns (bool) {
        return store.routes.has(bytes32(uint256(id.asUint())));
    }

    /**
     * @dev Get or create a value. O(1).
     *
     * Returns true if the value was added to the set, that is if it was not
     * already present.
     */
    function getOrAdd(MainStorage storage store, NetworkID id) private returns (bool, NetworkInfo storage) {
        (bool success, StoragePtr ptr) = store.routes.tryAdd(bytes32(uint256(id.asUint())));
        return (success, pointerToRoute(ptr));
    }

    /**
     * @dev Removes a value from a set. O(1).
     *
     * Returns true if the value was removed from the set, that is if it was
     * present.
     */
    function remove(MainStorage storage store, NetworkID id) internal returns (bool) {
        StoragePtr ptr = store.routes.remove(bytes32(uint256(id.asUint())));
        if (ptr.isNull()) {
            return false;
        }
        return true;
    }

    /**
     * @dev Returns the number of values on the set. O(1).
     */
    function length(MainStorage storage store) internal view returns (uint256) {
        return store.routes.length();
    }

    /**
     * @dev Returns the value stored at position `index` in the set. O(1).
     *
     * Note that there are no guarantees on the ordering of values inside the
     * array, and it may change when more values are added or removed.
     *
     * Requirements:
     *
     * - `index` must be strictly less than {length}.
     */
    function at(MainStorage storage store, uint256 index) internal view returns (NetworkID, NetworkInfo storage) {
        (bytes32 key, StoragePtr value) = store.routes.at(index);
        if (value.isNull()) {
            revert IndexOutOfBounds(index);
        }
        return (NetworkID.wrap(uint16(uint256(key))), pointerToRoute(value));
    }

    /**
     * @dev Returns the value associated with `NetworkInfo`. O(1).
     *
     * Requirements:
     * - `NetworkInfo` must be in the map.
     */
    function get(MainStorage storage store, NetworkID id) internal view returns (NetworkInfo storage) {
        StoragePtr ptr = store.routes.get(bytes32(uint256(id.asUint())));
        if (ptr.isNull()) {
            revert RouteNotExists(id);
        }
        return pointerToRoute(ptr);
    }

    /**
     * @dev Returns the value associated with `NetworkInfo`. O(1).
     */
    function tryGet(MainStorage storage store, NetworkID id) internal view returns (bool, NetworkInfo storage) {
        (bool exists, StoragePtr ptr) = store.routes.tryGet(bytes32(uint256(id.asUint())));
        return (exists, pointerToRoute(ptr));
    }

    function createOrUpdateRoute(MainStorage storage store, Route calldata route) internal {
        // Update network info
        (bool created, NetworkInfo storage stored) = getOrAdd(store, route.networkId);
        require((created && route.gateway != bytes32(0)) || !created, "domain separator cannot be zero");

        // Update gas limit if it's not zero
        if (route.gasLimit > 0) {
            stored.gasLimit = route.gasLimit;
        }

        // Update relative gas price and base fee if any of them are greater than zero
        if (route.relativeGasPriceDenominator > 0) {
            UFloat9x56 relativeGasPrice =
                UFloatMath.fromRational(route.relativeGasPriceNumerator, route.relativeGasPriceDenominator);
            stored.relativeGasPrice = relativeGasPrice;
            stored.baseFee = route.baseFee;
        }

        emit RouteUpdated(route.networkId.asUint(), stored.relativeGasPrice, stored.baseFee, stored.gasLimit);
    }

    /**
     * @dev Storage initializer function, used to set up the initial storage of the contract.
     * @param store Storage location.
     * @param networks List of networks to initialize.
     * @param networkdID The network id of this chain.
     */
    function initialize(MainStorage storage store, Network[] calldata networks, NetworkID networkdID) internal {
        for (uint256 i = 0; i < networks.length; i++) {
            Network calldata network = networks[i];
            (bool created, NetworkInfo storage info) = getOrAdd(store, NetworkID.wrap(network.id));
            require(created, "network already initialized");
            require(network.id != networkdID.asUint() || network.gateway == address(this), "wrong gateway address");
            info.gasLimit = 15_000_000; // Default to 15M gas
            info.relativeGasPrice = UFloatMath.ONE;
            info.baseFee = 0;
        }
    }

    /**
     * @dev Return all routes registered currently registered.
     *
     * WARNING: This operation will copy the entire storage to memory, which can be quite expensive. This is designed
     * to mostly be used by view accessors that are queried without any gas fees. Developers should keep in mind that
     * this function has an unbounded cost, and using it as part of a state-changing function may render the function
     * uncallable if the set grows to a point where copying to memory consumes too much gas to fit in a block.
     */
    function listRoutes(MainStorage storage store) internal view returns (Route[] memory) {
        bytes32[] memory idx = store.routes.keys;
        Route[] memory routes = new Route[](idx.length);
        for (uint256 i = 0; i < idx.length; i++) {
            (bool success, NetworkInfo storage route) = tryGet(store, NetworkID.wrap(uint16(uint256(idx[i]))));
            require(success, "route not found");
            (uint256 numerator, uint256 denominator) = route.relativeGasPrice.toRational();
            routes[i] = Route({
                networkId: NetworkID.wrap(uint16(uint256(idx[i]))),
                gasLimit: route.gasLimit,
                baseFee: route.baseFee,
                gateway: bytes32(uint256(uint160(address(this)))),
                relativeGasPriceNumerator: uint128(numerator),
                relativeGasPriceDenominator: uint128(denominator)
            });
        }
        return routes;
    }

    /**
     * @dev Check a few preconditions before estimate the GMP wei cost.
     */
    function _checkPreconditions(NetworkInfo memory route, uint256 messageSize, uint256 gasLimit) private pure {
        // Verify if the network exists
        require(route.baseFee > 0 || UFloat9x56.unwrap(route.relativeGasPrice) > 0, "route is temporarily disabled");

        // Verify if the gas limit and message size are within the limits
        require(gasLimit <= route.gasLimit, "gas limit exceeded");
        require(messageSize <= MAX_PAYLOAD_SIZE, "maximum payload size exceeded");
    }

    /**
     * @dev Utility function for measure the wei cost of a GMP message.
     */
    function estimateCost(NetworkInfo memory route, bytes calldata data, uint256 gasLimit)
        internal
        pure
        returns (uint256 gasCost, uint256 fee)
    {
        // Guarantee the networks exists and `data` is less than `MAX_PAYLOAD_SIZE`
        _checkPreconditions(route, data.length, gasLimit);

        // Compute base cost
        uint256 nonZeros = GasUtils.countNonZerosCalldata(data);
        uint256 zeros = data.length - nonZeros;

        // Compute execution cost
        gasCost = GasUtils.estimateGas(uint16(nonZeros), uint16(zeros), gasLimit);

        // Calculate the gas cost: gasPrice * gasCost + baseFee
        fee = UFloatMath.saturatingMul(route.relativeGasPrice, gasCost).saturatingAdd(route.baseFee);
    }

    /**
     * @dev Utility function for measure the wei cost of a GMP message.
     */
    function estimateWeiCost(NetworkInfo memory route, bytes calldata data, uint256 gasLimit)
        internal
        pure
        returns (uint256)
    {
        _checkPreconditions(route, data.length, gasLimit);
        uint256 nonZeros = GasUtils.countNonZerosCalldata(data);
        uint256 zeros = data.length - nonZeros;
        return
            GasUtils.estimateWeiCost(route.relativeGasPrice, route.baseFee, uint16(nonZeros), uint16(zeros), gasLimit);
    }

    /**
     * @dev Utility function for measure the wei cost of a GMP message.
     */
    function estimateWeiCost(NetworkInfo memory route, uint256 messageSize, uint256 gasLimit)
        internal
        pure
        returns (uint256)
    {
        _checkPreconditions(route, messageSize, gasLimit);
        return GasUtils.estimateWeiCost(route.relativeGasPrice, route.baseFee, uint16(messageSize), 0, gasLimit);
    }
}
