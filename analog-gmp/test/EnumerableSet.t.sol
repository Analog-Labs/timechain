// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/EnumerableSet.t.sol)

pragma solidity >=0.8.0;

import {Test, console} from "forge-std/Test.sol";
import {TestUtils} from "./TestUtils.sol";
import {EnumerableSet, StoragePtr} from "../src/utils/EnumerableSet.sol";
import {StoragePtr, Pointer} from "../src/utils/Pointer.sol";
import {BranchlessMath} from "../src/utils/BranchlessMath.sol";

contract EnumerableSetTest is Test {
    using BranchlessMath for uint256;
    using EnumerableSet for EnumerableSet.Map;
    using Pointer for StoragePtr;
    using Pointer for uint256;
    using Pointer for Pointer.Uint256Slot;

    uint256 private constant ITERATIONS = 10;

    EnumerableSet.Map private map;

    struct MyStruct {
        uint256 a;
        uint256 b;
        uint256 c;
    }

    function _add(uint256 key, uint256 value, bool success) private returns (MyStruct storage r) {
        bytes32 ptr = map.add(bytes32(key)).asBytes32();
        if (success) {
            assertNotEq(ptr, bytes32(0), "map.add failed");
            assembly {
                r.slot := ptr
            }
            r.a = value;
            r.b = value + 1;
            r.c = value + 2;
        } else {
            assertEq(ptr, bytes32(0), "expect map.add to fail");
            assembly {
                r.slot := 0
            }
        }
    }

    function _at(uint256 index, bool success) private view returns (MyStruct storage r) {
        (, StoragePtr raw) = map.at(index);
        bytes32 ptr = raw.asBytes32();
        if (success) {
            assertNotEq(ptr, bytes32(0), "map.at failed");
            assembly {
                r.slot := ptr
            }
        } else {
            assertEq(ptr, bytes32(0), "expect map.at to fail");
            assembly {
                r.slot := 0
            }
        }
    }

    function _get(uint256 key, bool success) private view returns (MyStruct storage r) {
        bytes32 ptr = map.get(bytes32(key)).asBytes32();
        if (success) {
            assertNotEq(ptr, bytes32(0), "map.at failed");
            assembly {
                r.slot := ptr
            }
        } else {
            assertEq(ptr, bytes32(0), "expect map.at to fail");
            assembly {
                r.slot := 0
            }
        }
    }

    function _removeByKey(uint256 key, bool success) private returns (MyStruct storage r) {
        bytes32 ptr = map.remove(bytes32(key)).asBytes32();
        if (success) {
            assertNotEq(ptr, bytes32(0), "map.at failed");
            assembly {
                r.slot := ptr
            }
        } else {
            assertEq(ptr, bytes32(0), "expect map.at to fail");
            assembly {
                r.slot := 0
            }
        }
    }

    /**
     * Test if `Map.add` and `Map.at` work as expected.
     */
    function test_add() external {
        assertEq(map.length(), 0, "Map should be empty");

        MyStruct storage s;
        for (uint256 i = 0; i < ITERATIONS; i++) {
            s = _add(0x1234 + i, i + 1, true);
            assertEq(map.length(), i + 1);
            for (uint256 j = 0; j < ITERATIONS; j++) {
                s = _at(j, j <= i);
                if (j <= i) {
                    assertEq(s.a, j + 1, "MyStruct.a mismatch");
                    assertEq(s.b, j + 2, "MyStruct.b mismatch");
                    assertEq(s.c, j + 3, "MyStruct.c mismatch");
                }
            }
        }
    }

    /**
     * Test if `Map.add` and `Map.at` work as expected.
     */
    function test_remove() external {
        assertEq(map.length(), 0, "Map should be empty");

        MyStruct storage s;
        for (uint256 i = 0; i < ITERATIONS; i++) {
            s = _add(0xdeadbeef + i, i + 1, true);
        }

        assertEq(map.length(), ITERATIONS, "unexpected map length");
        uint256 count = ITERATIONS - 1;
        _removeByKey(0xdeadbeef + count, true);
        assertEq(map.length(), count, "element not removed");

        // Cannot remove the same key twice
        _removeByKey(0xdeadbeef + count, false);

        // Cannot remove an unknown key
        _removeByKey(0xdeadbeef + ITERATIONS * 2, false);

        for (uint256 i = 0; i < ITERATIONS; i++) {
            s = _at(i, i < count);
            if (i < count) {
                assertEq(s.a, i + 1, "MyStruct.a mismatch");
                assertEq(s.b, i + 2, "MyStruct.b mismatch");
                assertEq(s.c, i + 3, "MyStruct.c mismatch");
            }
        }

        uint256 removeIndex = count - 3;
        _removeByKey(0xdeadbeef + removeIndex, true);
        count -= 1;
        assertEq(map.length(), count, "element not removed");
        s = _at(removeIndex, true);
        assertEq(s.a, count + 1, "MyStruct.a mismatch");
        assertEq(s.b, count + 2, "MyStruct.b mismatch");
        assertEq(s.c, count + 3, "MyStruct.c mismatch");

        s = _at(removeIndex + 1, true);
        assertEq(s.a, removeIndex + 2, "MyStruct.a mismatch");
        assertEq(s.b, removeIndex + 3, "MyStruct.b mismatch");
        assertEq(s.c, removeIndex + 4, "MyStruct.c mismatch");

        for (uint256 i = 0; i < map.keys.length; i++) {
            uint256 key = uint256(map.keys[i]);
            assertEq(map.values[bytes32(key)].asUint(), key - 0xdeadbeef + 1, "MyStruct.a mismatch");
            s = _get(key, true);
            assertEq(s.a, key - 0xdeadbeef + 1, "MyStruct.a mismatch");
            assertEq(s.b, key - 0xdeadbeef + 2, "MyStruct.b mismatch");
            assertEq(s.c, key - 0xdeadbeef + 3, "MyStruct.c mismatch");
        }
    }

    /**
     * Test if `Map.add` and `Map.at` work as expected.
     */
    function test_fuzz(bytes32 key, uint256 value) external {
        assertEq(map.length(), 0, "Map should be empty");

        // Map.length works
        Pointer.Uint256Slot storage store;
        store = map.add(key).getUint256Slot();
        assertFalse(store.asPtr().isNull(), "invalid pointer");
        store.value = value;

        // Map.length works
        assertEq(map.length(), 1, "unexpected map length");

        // Map.get works
        store = map.get(key).getUint256Slot();
        assertEq(store.value, value, "unexpected value when retrieving by key");

        // Map.indexOf works
        int256 index = map.indexOf(store.asPtr());
        assertEq(index, 0, "unexpected index");

        // Map.at works
        (bytes32 atKey, StoragePtr raw) = map.at(0);
        store = raw.getUint256Slot();
        assertEq(store.value, value, "unexpected value when retrieving by index");
        assertEq(atKey, key, "unexpected key when retrieving by index");

        // Map.contains works
        StoragePtr ptr = map.get(key);
        assertTrue(map.contains(ptr), "the key should be in the map");

        // Map.contains returns false for invalid pointers
        ptr = (ptr.asUint() + 1).asPtr();
        assertFalse(map.contains(ptr), "invalid pointer");
        ptr = (ptr.asUint() - 2).asPtr();
        assertFalse(map.contains(ptr), "invalid pointer");
    }
}
