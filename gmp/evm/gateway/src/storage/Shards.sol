// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/storage/Shards.sol)
pragma solidity ^0.8.20;

import {TssKey, Signature, PrimitiveUtils} from "../Primitives.sol";
import {EnumerableMap} from "@openzeppelin/contracts/utils/structs/EnumerableMap.sol";

/**
 * @dev EIP-7201 Shard's Storage
 */
library ShardStore {
    using EnumerableMap for EnumerableMap.UintToUintMap;
    using PrimitiveUtils for bool;

    /**
     * @dev Namespace of the shards storage `analog.one.gateway.shards`.
     * keccak256(abi.encode(uint256(keccak256("analog.one.gateway.shards")) - 1)) & ~bytes32(uint256(0xff));
     */
    bytes32 private constant _EIP7201_NAMESPACE = 0x582bcdebbeef4fb96dde802cfe96e9942657f4bedb5cfe94e8786bb683eb1f00;

    /**
     * @dev Emitted when shards are registered.
     * @param key registered shard's key
     */
    event ShardRegistered(TssKey key);

    /**
     * @dev Emitted when shards are unregistered.
     * @param key unregistered shard's key
     */
    event ShardRevoked(TssKey key);

    /**
     * @dev Shard info stored in the Gateway Contract
     * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
     * the shard info below 256 bit, so it can be stored in one single storage slot.
     * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
     *
     * @custom:storage-location erc7201:analog.one.gateway.shards
     */
    struct ShardInfo {
        uint8 yParity;
        uint16 numSessions;
    }

    /**
     * @dev Shard info stored in the Gateway Contract
     * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
     * the shard info below 256 bit, so it can be stored in one single storage slot.
     * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
     *
     */
    struct MainStorage {
        EnumerableMap.UintToUintMap shardIds;
        mapping(uint256 => ShardInfo) shards;
    }

    error ShardNotExists(uint256 xCoord);
    error InvalidYParity();
    error YParityMismatch();

    function getMainStorage() internal pure returns (MainStorage storage $) {
        assembly {
            $.slot := _EIP7201_NAMESPACE
        }
    }

    /**
     * @dev Returns the value associated with `key`. O(1).
     *
     * Requirements:
     * - `key` must be in the map.
     */
    function get(MainStorage storage store, uint256 xCoord) internal view returns (ShardInfo storage) {
        if (!store.shardIds.contains(xCoord)) {
            revert ShardNotExists(xCoord);
        }
        return store.shards[xCoord];
    }

    /**
     * @dev Register a single TSS key.
     * Requirements:
     * - The `newKey` should not be already registered.
     */
    function register(MainStorage storage store, TssKey calldata newKey) internal returns (bool) {
        // Check y-parity
        if (newKey.yParity != 27 && newKey.yParity != 28) {
            revert InvalidYParity();
        }

        ShardInfo storage stored = store.shards[newKey.xCoord];

        // Check if the shard is already registered
        if (store.shardIds.contains(newKey.xCoord)) {
            if (newKey.yParity != stored.yParity) {
                revert YParityMismatch();
            }
            return false;
        }

        store.shardIds.set(newKey.xCoord, 1);
        stored.yParity = newKey.yParity;
        stored.numSessions = newKey.numSessions;
        emit ShardRegistered(newKey);
        return true;
    }

    /**
     * @dev Revoke Shards keys.
     * Requirements:
     * - The `keys` must be registered.
     */
    function revoke(MainStorage storage store, TssKey calldata key) internal returns (bool) {
        // check exists
        if (!store.shardIds.contains(key.xCoord)) {
            return false;
        }

        // Check y-parity
        ShardInfo storage stored = store.shards[key.xCoord];
        if (stored.yParity != key.yParity) {
            revert YParityMismatch();
        }

        store.shardIds.remove(key.xCoord);
        delete store.shards[key.xCoord];
        emit ShardRevoked(key);
        return true;
    }

    /**
     * @dev Return all shards registered currently registered.
     *
     * WARNING: This operation will copy the entire storage to memory, which can be quite expensive. This is designed
     * to mostly be used by view accessors that are queried without any gas fees. Developers should keep in mind that
     * this function has an unbounded cost, and using it as part of a state-changing function may render the function
     * uncallable if the set grows to a point where copying to memory consumes too much gas to fit in a block.
     */
    function list(MainStorage storage store) internal view returns (TssKey[] memory) {
        uint256 len = store.shardIds.length();
        TssKey[] memory shards = new TssKey[](len);

        for (uint256 i = 0; i < len; i++) {
            (uint256 key,) = store.shardIds.at(i);
            ShardInfo storage shard = store.shards[key];
            shards[i] = TssKey({yParity: shard.yParity, xCoord: key, numSessions: shard.numSessions});
        }
        return shards;
    }
}
