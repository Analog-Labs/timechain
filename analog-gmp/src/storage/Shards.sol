// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/storage/Shards.sol)
pragma solidity ^0.8.20;

import {TssKey, Signature} from "../Primitives.sol";
import {EnumerableSet, Pointer} from "../utils/EnumerableSet.sol";
import {BranchlessMath} from "../utils/BranchlessMath.sol";
import {StoragePtr} from "../utils/Pointer.sol";

library _ShardStore {
    function from(uint256 xCoord) internal pure returns (ShardStore.ShardID) {
        return ShardStore.ShardID.wrap(bytes32(xCoord));
    }

    /**
     * @dev Converts a `StoragePtr` into a `ShardInfo`.
     */
    function asShardInfo(StoragePtr ptr) internal pure returns (ShardStore.ShardInfo storage info) {
        assembly {
            info.slot := ptr
        }
    }

    /**
     * @dev Converts a `ShardInfo` into a `StoragePtr`.
     */
    function asPtr(ShardStore.ShardInfo storage info) internal pure returns (StoragePtr ptr) {
        assembly {
            ptr := info.slot
        }
    }
}

/**
 * @dev EIP-7201 Shard's Storage
 */
library ShardStore {
    using Pointer for StoragePtr;
    using Pointer for uint256;
    using EnumerableSet for EnumerableSet.Map;
    using _ShardStore for uint256;
    using _ShardStore for StoragePtr;
    using _ShardStore for ShardInfo;

    /**
     * @dev Namespace of the shards storage `analog.one.gateway.shards`.
     * keccak256(abi.encode(uint256(keccak256("analog.one.gateway.shards")) - 1)) & ~bytes32(uint256(0xff));
     */
    bytes32 internal constant _EIP7201_NAMESPACE = 0x582bcdebbeef4fb96dde802cfe96e9942657f4bedb5cfe94e8786bb683eb1f00;

    uint8 internal constant SHARD_ACTIVE = (1 << 0); // Shard active bitflag
    uint8 internal constant SHARD_Y_PARITY = (1 << 1); // Pubkey y parity bitflag

    /**
     * @dev Shard ID, this is the xCoord of the TssKey
     */
    type ShardID is bytes32;

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
        uint32 nonce;
        uint64 createdAtBlock;
    }

    /**
     * @dev Shard info stored in the Gateway Contract
     * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
     * the shard info below 256 bit, so it can be stored in one single storage slot.
     * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
     *
     * @custom:storage-location erc7201:analog.one.gateway.shards
     */
    struct MainStorage {
        EnumerableSet.Map shards;
    }

    error ShardAlreadyRegistered(ShardID id);
    error ShardNotExists(ShardID id);
    error IndexOutOfBounds(uint256 index);

    function getMainStorage() internal pure returns (MainStorage storage $) {
        assembly {
            $.slot := _EIP7201_NAMESPACE
        }
    }

    /**
     * @dev Returns true if the value is in the set. O(1).
     */
    function has(MainStorage storage store, ShardID id) internal view returns (bool) {
        return store.shards.has(ShardID.unwrap(id));
    }

    /**
     * @dev Get or create a value. O(1).
     *
     * Returns true if the value was added to the set, that is if it was not
     * already present.
     */
    function getOrAdd(MainStorage storage store, ShardID xCoord) private returns (bool, ShardInfo storage) {
        (bool success, StoragePtr ptr) = store.shards.tryAdd(ShardID.unwrap(xCoord));
        return (success, ptr.asShardInfo());
    }

    /**
     * @dev Removes a value from a set. O(1).
     *
     * Reverts if the value does not exist in the set.
     */
    function remove(MainStorage storage store, ShardID id) internal {
        StoragePtr ptr = store.shards.remove(ShardID.unwrap(id));
        if (ptr.isNull()) {
            revert ShardNotExists(id);
        }
    }

    /**
     * @dev Returns the number of values on the set. O(1).
     */
    function length(MainStorage storage store) internal view returns (uint256) {
        return store.shards.length();
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
    function at(MainStorage storage store, uint256 index) internal view returns (ShardID, ShardInfo storage) {
        (bytes32 xCoord, StoragePtr ptr) = store.shards.at(index);
        if (ptr.isNull()) {
            revert IndexOutOfBounds(index);
        }
        return (ShardID.wrap(xCoord), ptr.asShardInfo());
    }

    /**
     * @dev Returns the value associated with `key`. O(1).
     *
     * Requirements:
     * - `key` must be in the map.
     */
    function get(MainStorage storage store, ShardID key) internal view returns (ShardInfo storage) {
        StoragePtr ptr = store.shards.get(ShardID.unwrap(key));
        if (ptr.isNull()) {
            revert ShardNotExists(key);
        }
        return ptr.asShardInfo();
    }

    /**
     * @dev Returns the `KeyInfo` associated with `TssKey`. O(1).
     *
     * Requirements:
     * - `key.xCoord` must be in the map.
     */
    function get(MainStorage storage store, TssKey calldata key) internal view returns (ShardInfo storage) {
        return get(store, ShardID.wrap(bytes32(key.xCoord)));
    }

    /**
     * @dev Returns the `KeyInfo` associated with `Signature`. O(1).
     *
     * Requirements:
     * - `signature.xCoord` must be in the map.
     */
    function get(MainStorage storage store, Signature calldata signature) internal view returns (ShardInfo storage) {
        return get(store, ShardID.wrap(bytes32(signature.xCoord)));
    }

    /**
     * @dev Returns the value associated with `key`. O(1).
     */
    function tryGet(MainStorage storage store, ShardID key) private view returns (bool, ShardInfo storage) {
        (bool exists, StoragePtr ptr) = store.shards.tryGet(ShardID.unwrap(key));
        return (exists, ptr.asShardInfo());
    }

    /**
     * @dev Register a single TSS key.
     * Requirements:
     * - The `newKey` should not be already registered.
     */
    function register(MainStorage storage store, TssKey calldata newKey) internal returns (bool) {
        // Check y-parity
        require((newKey.yParity == 2 || newKey.yParity == 3), "y parity bit must be 2 or 3, cannot register shard");

        // Read shard from storage
        ShardID id = ShardID.wrap(bytes32(newKey.xCoord));
        (bool created, ShardInfo storage stored) = getOrAdd(store, id);

        // Check if the shard is already registered
        if (!created) {
            require(stored.nonce == 1 || newKey.yParity == (stored.yParity | 2), "tsskey.yParity mismatch");
            return false;
        }

        // Get the current status and nonce
        ShardInfo memory shard = stored;

        require(
            shard.createdAtBlock == 0 || (shard.yParity | 2) == newKey.yParity,
            "the provided y-parity doesn't match the existing y-parity, cannot register shard"
        );

        // Update nonce
        shard.nonce |= uint32(BranchlessMath.toUint(shard.nonce == 0));

        // Save new status and nonce in the storage
        stored.createdAtBlock =
            BranchlessMath.ternaryU64(shard.createdAtBlock > 0, shard.createdAtBlock, uint64(block.number));
        stored.nonce = shard.nonce;
        stored.yParity = newKey.yParity & 1;
        return true;
    }

    /**
     * @dev Register TSS keys in batch.
     * Requirements:
     * - The `keys` should not be already registered.
     */
    function registerTssKeys(MainStorage storage store, TssKey[] calldata keys) internal {
        // We don't perform any arithmetic operation, except iterate a loop
        unchecked {
            // Register or activate tss key (revoked keys keep the previous nonce)
            for (uint256 i = 0; i < keys.length; i++) {
                register(store, keys[i]);
            }
        }
    }

    /**
     * @dev Replace TSS keys in batch.
     * Requirements:
     * - The `keys` may or may not be registered.
     */
    function replaceTssKeys(MainStorage storage store, TssKey[] calldata keys)
        internal
        returns (TssKey[] memory created, TssKey[] memory revoked)
    {
        unchecked {
            revoked = listShards(store);
            created = new TssKey[](keys.length);

            // Make sure the tss keys are correctly ordered, this makes easier to prevent repeated keys, and
            // allows binary search.
            uint256 createdCount = 0;
            for (uint256 i = 0; i < keys.length; i++) {
                TssKey calldata key = keys[i];
                if (i > 0) {
                    TssKey calldata previousKey = keys[i - 1];
                    require(
                        previousKey.xCoord < key.xCoord, "tss keys must be orderd by 'key.xCoord' in asceding order"
                    );
                }

                if (register(store, key)) {
                    // Shard registered
                    created[createdCount++] = TssKey({yParity: key.yParity, xCoord: key.xCoord});
                } else {
                    // Shard already registered, remove it from the revoke list.
                    uint256 len = revoked.length;
                    for (uint256 j = 0; j < len; j++) {
                        TssKey memory current = revoked[j];
                        if (current.xCoord == key.xCoord) {
                            revoked[j] = revoked[len - 1];
                            len--;
                            assembly {
                                // decrement list, equivalent to `revoked.length--`
                                mstore(revoked, len)
                            }
                        }
                    }
                }
            }

            // Update `created` list length
            assembly {
                mstore(created, createdCount)
            }

            // Revoke Shards
            for (uint256 i = 0; i < revoked.length; i++) {
                TssKey memory key = revoked[i];
                _revoke(store, ShardID.wrap(bytes32(key.xCoord)));
            }
        }
    }

    /**
     * @dev Revoke Shards keys.
     * Requirements:
     * - The `keys` must be registered.
     */
    function revoke(MainStorage storage store, TssKey calldata key) internal returns (bool) {
        // Read shard from storage
        ShardID id = ShardID.wrap(bytes32(key.xCoord));
        (bool exists, ShardInfo memory stored) = tryGet(store, id);

        if (exists) {
            // Check y-parity
            require(stored.yParity == (key.yParity & 1), "y parity mismatch, cannot revoke key");
            return _revoke(store, id);
        }
        return false;
    }

    /**
     * @dev Revoke Shards keys.
     */
    function _revoke(MainStorage storage store, ShardID id) private returns (bool) {
        // Remove from the set
        StoragePtr ptr = store.shards.remove(ShardID.unwrap(id));
        return !ptr.isNull();
    }

    /**
     * @dev Revoke TSS keys im batch.
     * Requirements:
     * - The `publicKeys` must be registered.
     */
    function revokeKeys(MainStorage storage store, TssKey[] calldata publicKeys)
        internal
        returns (TssKey[] memory revokedKeys)
    {
        // Revoke tss keys
        uint256 keysLength = publicKeys.length;
        revokedKeys = new TssKey[](keysLength);
        uint256 revokedCount = 0;

        for (uint256 i = 0; i < publicKeys.length; i++) {
            if (revoke(store, publicKeys[i])) {
                revokedKeys[revokedCount++] = publicKeys[i];
            }
        }

        if (revokedKeys.length != keysLength) {
            assembly {
                mstore(revokedKeys, revokedCount)
            }
        }
        return revokedKeys;
    }

    function _t(MainStorage storage store) internal view returns (TssKey[] memory) {}

    /**
     * @dev Return all shards registered currently registered.
     *
     * WARNING: This operation will copy the entire storage to memory, which can be quite expensive. This is designed
     * to mostly be used by view accessors that are queried without any gas fees. Developers should keep in mind that
     * this function has an unbounded cost, and using it as part of a state-changing function may render the function
     * uncallable if the set grows to a point where copying to memory consumes too much gas to fit in a block.
     */
    function listShards(MainStorage storage store) internal view returns (TssKey[] memory) {
        bytes32[] storage idx = store.shards.keys;
        uint256 len = idx.length;
        TssKey[] memory shards = new TssKey[](len);
        for (uint256 i = 0; i < len; i++) {
            ShardID id = ShardID.wrap(idx[i]);
            (bool success, ShardInfo storage shard) = tryGet(store, id);
            if (!success) {
                revert ShardNotExists(id);
            }
            shards[i] = TssKey(shard.yParity + 2, uint256(ShardID.unwrap(id)));
        }
        return shards;
    }
}
