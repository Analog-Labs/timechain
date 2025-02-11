// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/utils/EnumerableMap.sol)
pragma solidity ^0.8.20;

import {StoragePtr, Pointer} from "./Pointer.sol";

/**
 * @dev Library for managing an enumerable variant of Solidity's
 * https://solidity.readthedocs.io/en/latest/types.html#mapping-types[`mapping`]
 * type.
 */
library EnumerableSet {
    using Pointer for StoragePtr;

    error ValueAlreadyPresent(bytes32);

    /**
     * @dev Shard info stored in the Gateway Contract
     * OBS: the order of the attributes matters! ethereum storage is 256bit aligned, try to keep
     * the shard info below 256 bit, so it can be stored in one single storage slot.
     * reference: https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html
     *
     * @custom:storage-location erc7201:analog.one.gateway.shards
     */
    struct Map {
        bytes32[] keys;
        mapping(bytes32 => StoragePtr) values;
    }

    /**
     * @dev Returns index of a given value in the set. O(1).
     *
     * Returns -1 if the value is not in the set.
     */
    function indexOf(Map storage map, StoragePtr ptr) internal view returns (int256 index) {
        assembly ("memory-safe") {
            index := not(sload(sub(ptr, 1)))
            mstore(0x00, map.slot)
            mstore(0x00, sload(add(keccak256(0x00, 0x20), index)))
            mstore(0x20, add(map.slot, 1))
            index := or(index, sub(and(eq(ptr, keccak256(0x00, 0x40)), lt(index, sload(map.slot))), 1))
        }
    }

    /**
     * @dev Returns true if the value is in the set. O(1).
     */
    function contains(Map storage map, StoragePtr value) internal view returns (bool r) {
        return indexOf(map, value) >= 0;
    }

    /**
     * @dev Returns true if the key is in the set. O(1).
     */
    function has(Map storage map, bytes32 key) internal view returns (bool r) {
        assembly ("memory-safe") {
            mstore(0x00, key)
            mstore(0x20, add(map.slot, 1))
            r := keccak256(0x00, 0x40)
            r := gt(sload(sub(r, 1)), 0)
        }
    }

    /**
     * @dev Add a value to a set. O(1).
     *
     * Returns true if the value was added to the set, that is if it was not
     * already present.
     */
    function add(Map storage map, bytes32 key) internal returns (StoragePtr r) {
        bool success;
        (success, r) = tryAdd(map, key);
        if (!success) {
            revert ValueAlreadyPresent(key);
        }
    }

    /**
     * @dev Add a value to a set. O(1).
     *
     * Returns true if the value was added to the set, that is if it was not
     * already present.
     */
    function tryAdd(Map storage map, bytes32 key) internal returns (bool success, StoragePtr r) {
        assembly ("memory-safe") {
            mstore(0x00, key)
            mstore(0x20, add(map.slot, 1))
            r := keccak256(0x00, 0x40)
            success := iszero(sload(sub(r, 1)))
            if success {
                // Load the array size
                let size := sload(map.slot)

                // Store the value
                mstore(0x00, map.slot)
                sstore(add(keccak256(0x00, 0x20), size), key)

                // Update the value's index
                sstore(sub(r, 1), not(size))

                // Update array size
                size := add(size, 1)
                sstore(map.slot, size)
            }
        }
    }

    /**
     * @dev Removes a value from a set. O(1).
     *
     * Returns the removed value storage pointer, if it was present, or null if it was not.
     */
    function remove(Map storage map, bytes32 key) internal returns (StoragePtr r) {
        assembly ("memory-safe") {
            // Find the value's index
            mstore(0x00, key)
            mstore(0x20, add(map.slot, 1))
            r := keccak256(0x00, 0x40)
            let index := not(sload(sub(r, 1)))

            // First element storage index
            let keys_count := sload(map.slot)
            mstore(0x00, map.slot)
            let keys_start := keccak256(0x00, 0x20)
            let val_key_ptr := add(keys_start, index)

            // (index < map.keys.length) && key == map.keys[map.values[key].index]
            r := mul(r, and(lt(index, keys_count), eq(key, sload(val_key_ptr))))

            if r {
                // (index + 1) < map.keys.length
                if lt(add(index, 1), keys_count) {
                    // Move the last element to the removed element's position
                    let last_index := sub(keys_count, 1)
                    let last_key := sload(add(keys_start, last_index))
                    sstore(val_key_ptr, last_key)

                    // Update the last element's index
                    mstore(0x00, last_key)
                    mstore(0x20, add(map.slot, 1))
                    sstore(sub(keccak256(0x00, 0x40), 1), not(index))
                }

                // Update array size
                sstore(map.slot, sub(keys_count, 1))

                // Remove index
                sstore(sub(r, 1), 0)
            }
        }
    }

    /**
     * @dev Returns the number of values on the set. O(1).
     */
    function length(Map storage map) internal view returns (uint256) {
        return map.keys.length;
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
    function at(Map storage map, uint256 index) internal view returns (bytes32 key, StoragePtr r) {
        assembly ("memory-safe") {
            mstore(0x00, map.slot)
            key := sload(add(keccak256(0x00, 0x20), index))
            mstore(0x00, key)
            mstore(0x20, add(map.slot, 1))
            r := keccak256(0x00, 0x40)
            r := mul(r, and(lt(index, sload(map.slot)), eq(index, not(sload(sub(r, 1))))))
        }
    }

    /**
     * @dev Returns the value associated with `key`. O(1).
     *
     * Requirements:
     * - `key` must be in the map.
     */
    function get(Map storage map, bytes32 key) internal view returns (StoragePtr r) {
        assembly ("memory-safe") {
            mstore(0x00, key)
            mstore(0x20, add(map.slot, 1))
            r := keccak256(0x00, 0x40)
            r := mul(r, gt(sload(sub(r, 1)), 0))
        }
    }

    /**
     * @dev Tries to returns the value associated with `key`. O(1).
     * Does not revert if `key` is not in the map.
     */
    function tryGet(Map storage map, bytes32 key) internal view returns (bool exists, StoragePtr r) {
        assembly ("memory-safe") {
            mstore(0x00, key)
            mstore(0x20, add(map.slot, 1))
            r := keccak256(0x00, 0x40)
            exists := gt(sload(sub(r, 1)), 0)
        }
    }

    /**
     * @dev Returns the value associated with `key`. O(1).
     */
    function getUnchecked(Map storage map, bytes32 key) internal pure returns (StoragePtr r) {
        assembly ("memory-safe") {
            mstore(0x00, key)
            mstore(0x20, add(map.slot, 1))
            r := keccak256(0x00, 0x40)
        }
    }

    // /**
    //  * @dev Return the entire set in an array
    //  *
    //  * WARNING: This operation will copy the entire storage to memory, which can be quite expensive. This is designed
    //  * to mostly be used by view accessors that are queried without any gas fees. Developers should keep in mind that
    //  * this function has an unbounded cost, and using it as part of a state-changing function may render the function
    //  * uncallable if the set grows to a point where copying to memory consumes too much gas to fit in a block.
    //  */
    // function _values(EnumerableMap storage m) private view returns (KeyInfo[] memory) {
    //     ShardID[] memory keys = s.keys;
    //     KeyInfo[] memory values = new KeyInfo[](keys.length);
    //     for (uint256 i = 0; i < keys.length; i++) {
    //         values[i] = s.shards[keys[i]];
    //     }
    //     return values;
    // }
}
