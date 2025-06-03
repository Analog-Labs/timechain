// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/storage/Routes.sol)
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {ShardStore} from "../src/storage/Shards.sol";
import {TssKey} from "../src/Primitives.sol";
import {Signer} from "frost-evm/Signer.sol";

contract ShardStoreTest is Test {
    using ShardStore for ShardStore.MainStorage;

    TssKey[3] keys;
    TssKey invalidKey;

    function setUp() public {
        // 4 gives parity 27
        // 5 gives parity 28
        for (uint16 i = 4; i <= 6; i++) {
            Signer signer = new Signer(i + 1);
            keys[i - 4] = TssKey({yParity: signer.yParity(), xCoord: signer.xCoord(), numSessions: 1});
        }
        invalidKey = TssKey({yParity: keys[2].yParity - 2, xCoord: keys[2].xCoord, numSessions: 1});
    }

    function getStore() internal pure returns (ShardStore.MainStorage storage) {
        return ShardStore.getMainStorage();
    }

    function externalGet(uint256 xCoord) external view returns (ShardStore.ShardInfo memory) {
        return getStore().get(xCoord);
    }

    function externalRegister(TssKey calldata key) external returns (bool) {
        return getStore().register(key);
    }

    function externalRevoke(TssKey calldata key) external returns (bool) {
        return getStore().revoke(key);
    }

    function register(TssKey memory key) internal returns (bool) {
        bytes memory callData = abi.encodeWithSelector(this.externalRegister.selector, key);
        (bool success, bytes memory returnData) = address(this).call(callData);
        require(success, "Register call failed");
        return abi.decode(returnData, (bool));
    }

    function revoke(TssKey memory key) internal returns (bool) {
        bytes memory callData = abi.encodeWithSelector(this.externalRevoke.selector, key);
        (bool success, bytes memory returnData) = address(this).call(callData);
        require(success, "Revoke call failed");
        return abi.decode(returnData, (bool));
    }

    function get(uint256 xCoord) internal returns (ShardStore.ShardInfo memory) {
        bytes memory callData = abi.encodeWithSelector(this.externalGet.selector, xCoord);
        (bool success, bytes memory returnData) = address(this).call(callData);
        require(success, "Get call failed");
        return abi.decode(returnData, (ShardStore.ShardInfo));
    }

    /// Tests
    function testRegisterNewShard() public {
        uint256 xCoord = keys[0].xCoord;
        uint8 y_parity = keys[0].yParity;
        TssKey memory newKey = TssKey({yParity: y_parity, xCoord: xCoord, numSessions: 1});

        bool registered = register(newKey);
        assertTrue(registered, "New shard should be registered");

        ShardStore.ShardInfo memory stored = get(xCoord);
        assertEq(stored.yParity, y_parity, "Y-parity mismatch");
    }

    function testRegisterExistingShard() public {
        testRegisterNewShard();

        TssKey memory existingKey = keys[0];

        bool registered = register(existingKey);
        assertFalse(registered, "Existing shard should not be re-registered");
        assertEq(getStore().list().length, 1, "Store length should remain unchanged");
    }

    function testRegisterInvalidYParity() public {
        vm.expectRevert(ShardStore.InvalidYParity.selector);
        register(invalidKey);
    }

    function testRegisterTssKeysBatch() public {
        for (uint256 i = 0; i < keys.length; i++) {
            register(keys[i]);
        }

        assertEq(getStore().list().length, 3, "All keys should be registered");
        for (uint256 i = 0; i < keys.length; i++) {
            get(keys[i].xCoord);
        }
    }

    function testRevokeShard() public {
        testRegisterNewShard();

        TssKey memory keyToRevoke = TssKey({yParity: keys[0].yParity, xCoord: keys[0].xCoord, numSessions: 1});

        bool revoked = revoke(keyToRevoke);
        assertTrue(revoked, "Key should be revoked");

        vm.expectRevert();
        get(keys[0].xCoord);
        assertEq(getStore().list().length, 0, "Store should be empty");
    }

    function testRevokeNonExistentShard() public {
        TssKey memory nonExistentKey = TssKey({yParity: keys[0].yParity, xCoord: keys[0].xCoord, numSessions: 1});

        bool revoked = revoke(nonExistentKey);
        assertFalse(revoked, "Non-existent key revocation should return false");
    }

    function testRevokeWithWrongYParity() public {
        TssKey memory originalKey = TssKey({yParity: keys[0].yParity, xCoord: keys[0].xCoord, numSessions: 1});
        register(originalKey);

        TssKey memory wrongParityKey = TssKey({yParity: keys[0].yParity - 2, xCoord: keys[0].xCoord, numSessions: 1});

        vm.expectRevert(ShardStore.YParityMismatch.selector);
        revoke(wrongParityKey);
    }

    function testGetNonExistentShard() public {
        uint256 nonExistentId = keys[0].xCoord;
        vm.expectRevert(abi.encodeWithSelector(ShardStore.ShardNotExists.selector, nonExistentId));
        get(nonExistentId);
    }

    function testListShards() public {
        TssKey[] memory newKeys = new TssKey[](3);
        newKeys[0] = TssKey({yParity: keys[0].yParity, xCoord: keys[0].xCoord, numSessions: 1});
        newKeys[1] = TssKey({yParity: keys[1].yParity, xCoord: keys[1].xCoord, numSessions: 1});
        newKeys[2] = TssKey({yParity: keys[2].yParity, xCoord: keys[2].xCoord, numSessions: 1});
        register(newKeys[0]);
        register(newKeys[1]);
        register(newKeys[2]);

        TssKey[] memory listed = getStore().list();
        assertEq(listed.length, 3, "Should list all shards");

        bool[] memory found = new bool[](3);
        for (uint256 i = 0; i < listed.length; i++) {
            for (uint256 j = 0; j < newKeys.length; j++) {
                if (listed[i].xCoord == newKeys[j].xCoord && listed[i].yParity == newKeys[j].yParity) {
                    found[j] = true;
                    break;
                }
            }
        }

        for (uint256 i = 0; i < found.length; i++) {
            assertTrue(found[i], "All original keys should be found in listing");
        }
    }
}
