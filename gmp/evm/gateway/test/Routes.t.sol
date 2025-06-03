// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/storage/Routes.sol)
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {RouteStore} from "../src/storage/Routes.sol";
import {Route} from "../src/Primitives.sol";

contract RouteStoreTest is Test {
    using RouteStore for RouteStore.MainStorage;

    uint16 constant TEST_NETWORK_ID = 1;
    bytes32 constant TEST_GATEWAY = bytes32(uint256(0x1));
    uint64 constant TEST_GAS_LIMIT = 500_000;
    uint128 constant TEST_BASE_FEE = 0.01 ether;
    uint256 constant TEST_NUMERATOR = 15;
    uint256 constant TEST_DENOMINATOR = 10;
    uint64 constant TEST_GAS_COEFF0 = 10;
    uint64 constant TEST_GAS_COEFF1 = 10;

    function getStore() internal pure returns (RouteStore.MainStorage storage) {
        return RouteStore.getMainStorage();
    }

    function externalInsert(Route calldata route) external {
        getStore().insert(route);
    }

    function externalEstimateCost(RouteStore.NetworkInfo memory route, bytes calldata data, uint256 gasLimit)
        external
        pure
        returns (uint256)
    {
        uint256 gas = RouteStore.estimateGas(route, uint16(data.length), uint64(gasLimit));
        return RouteStore.estimateCost(route, gas);
    }

    function externalGet(uint16 networkId) external view {
        getStore().get(networkId);
    }

    function insertRouteCall(Route memory route) internal {
        bytes memory callData = abi.encodeWithSelector(this.externalInsert.selector, route);
        (bool success,) = address(this).call(callData);
        require(success, "Call failed");
    }

    function getRoute() internal pure returns (Route memory route) {
        route = Route({
            networkId: TEST_NETWORK_ID,
            gateway: TEST_GATEWAY,
            gasLimit: TEST_GAS_LIMIT,
            baseFee: TEST_BASE_FEE,
            relativeGasPriceNumerator: TEST_NUMERATOR,
            relativeGasPriceDenominator: TEST_DENOMINATOR,
            gasCoef0: TEST_GAS_COEFF0,
            gasCoef1: TEST_GAS_COEFF1
        });
    }

    function testCreateNewRoute() public {
        Route memory newRoute = getRoute();

        insertRouteCall(newRoute);

        RouteStore.NetworkInfo memory stored = getStore().get(TEST_NETWORK_ID);
        assertEq(stored.gateway, TEST_GATEWAY, "Gateway mismatch");
        getStore().get(TEST_NETWORK_ID);
    }

    function testUpdateExistingRoute() public {
        testCreateNewRoute();

        Route memory updatedRoute = getRoute();
        updatedRoute.gasLimit = updatedRoute.gasLimit * 2;
        updatedRoute.baseFee = updatedRoute.gasLimit * 2;
        updatedRoute.relativeGasPriceNumerator = updatedRoute.relativeGasPriceNumerator * 3;

        vm.expectEmit(true, true, true, true);
        emit RouteStore.RouteUpdated(
            updatedRoute.networkId,
            updatedRoute.relativeGasPriceNumerator,
            updatedRoute.relativeGasPriceDenominator,
            updatedRoute.baseFee,
            updatedRoute.gasLimit,
            updatedRoute.gasCoef0,
            updatedRoute.gasCoef1
        );

        insertRouteCall(updatedRoute);

        RouteStore.NetworkInfo memory stored = getStore().get(TEST_NETWORK_ID);
        assertEq(stored.gasLimit, updatedRoute.gasLimit, "Gas limit update failed");
        assertEq(stored.baseFee, updatedRoute.baseFee, "Base fee update failed");
    }

    function testListRoutes() public {
        uint8 numRoutes = 5;
        for (uint16 i = 1; i <= numRoutes; i++) {
            Route memory r = Route({
                networkId: i,
                gateway: bytes32(uint256(i)),
                gasLimit: uint64(i) * 100_000,
                baseFee: uint128(i) * 0.01 ether,
                relativeGasPriceNumerator: i * 2,
                relativeGasPriceDenominator: i * 3,
                gasCoef0: i * 3,
                gasCoef1: i * 3
            });
            insertRouteCall(r);
        }

        Route[] memory routes = getStore().list();
        assertEq(routes.length, numRoutes, "Route count mismatch");

        for (uint16 i = 0; i < numRoutes; i++) {
            uint16 expectedId = i + 1;
            assertEq(routes[i].networkId, expectedId, "Network ID order mismatch");
            assertEq(routes[i].gasLimit, uint64(expectedId) * 100_000, "Gas limit order mismatch");
        }
    }

    function testEmptyStore() public view {
        Route[] memory routes = getStore().list();
        assertEq(routes.length, 0, "Should return empty array");
    }

    function testGetNonExistentRoute() public {
        vm.expectRevert(abi.encodeWithSelector(RouteStore.RouteNotExists.selector, TEST_NETWORK_ID));
        this.externalGet(TEST_NETWORK_ID);
    }

    function testEstimateCost() public {
        testCreateNewRoute();
        RouteStore.NetworkInfo memory route = getStore().get(TEST_NETWORK_ID);

        bytes memory payload = hex"deadbeef";
        uint256 fee = this.externalEstimateCost(route, payload, 100_000);
        assertGt(fee, 0, "Fee not calculated");
    }
}
