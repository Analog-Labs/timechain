// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/utils/StoragePtr.sol)
pragma solidity ^0.8.20;

/**
 * @dev Represents a raw pointer to a value in storage.
 */
type StoragePtr is uint256;

/**
 * @dev Library for reading and writing primitive types to specific storage slots.
 *
 * Storage slots are often used to avoid storage conflict when dealing with upgradeable contracts.
 * This library helps with reading and writing to such slots without the need for inline assembly.
 *
 * The functions in this library return Slot structs that contain a `value` member that can be used to read or write.
 *
 * Example usage to set ERC-1967 implementation slot:
 * ```solidity
 * contract ERC1967 {
 *     // Define the slot. Alternatively, use the SlotDerivation library to derive the slot.
 *     bytes32 internal constant _IMPLEMENTATION_SLOT = 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;
 *
 *     function _getImplementation() internal view returns (address) {
 *         return StorageSlot.getAddressSlot(_IMPLEMENTATION_SLOT).value;
 *     }
 *
 *     function _setImplementation(address newImplementation) internal {
 *         require(newImplementation.code.length > 0);
 *         StorageSlot.getAddressSlot(_IMPLEMENTATION_SLOT).value = newImplementation;
 *     }
 * }
 * ```
 *
 * TIP: Consider using this library along with {SlotDerivation}.
 */
library Pointer {
    struct AddressSlot {
        address value;
    }

    struct BooleanSlot {
        bool value;
    }

    struct Bytes32Slot {
        bytes32 value;
    }

    struct Uint256Slot {
        uint256 value;
    }

    struct Int256Slot {
        int256 value;
    }

    struct StringSlot {
        string value;
    }

    struct BytesSlot {
        bytes value;
    }

    /**
     * @dev Converts `uint256[] storage` to `StoragePtr`.
     */
    function asPtr(uint256[] storage store) internal pure returns (StoragePtr ptr) {
        assembly ("memory-safe") {
            ptr := store.slot
        }
    }

    /**
     * @dev Converts `bytes32[] storage` to `StoragePtr`.
     */
    function asPtr(bytes32[] storage store) internal pure returns (StoragePtr ptr) {
        assembly ("memory-safe") {
            ptr := store.slot
        }
    }

    /**
     * @dev Converts `bytes storage` to `StoragePtr`.
     */
    function asPtr(bytes storage store) internal pure returns (StoragePtr ptr) {
        assembly ("memory-safe") {
            ptr := store.slot
        }
    }

    /**
     * @dev Wraps a value in a `StoragePtr`.
     */
    function asPtr(uint256 value) internal pure returns (StoragePtr) {
        return StoragePtr.wrap(value);
    }

    /**
     * @dev Unwraps a `StoragePtr` to a value.
     */
    function asPtr(bytes32 value) internal pure returns (StoragePtr) {
        return StoragePtr.wrap(uint256(value));
    }

    /**
     * @dev Convert a `StoragePtr` to `uint256`.
     */
    function asUint(StoragePtr ptr) internal pure returns (uint256) {
        return StoragePtr.unwrap(ptr);
    }

    /**
     * @dev Convert a `StoragePtr` to `int256`.
     */
    function asInt(StoragePtr ptr) internal pure returns (int256) {
        return int256(StoragePtr.unwrap(ptr));
    }

    /**
     * @dev Convert a `StoragePtr` to `bytes32`.
     */
    function asBytes32(StoragePtr ptr) internal pure returns (bytes32) {
        return bytes32(StoragePtr.unwrap(ptr));
    }

    /**
     * @dev Whether the `StoragePtr` is zero or not.
     */
    function isNull(StoragePtr ptr) internal pure returns (bool r) {
        assembly ("memory-safe") {
            r := iszero(ptr)
        }
    }

    /**
     * @dev Returns an `AddressSlot` with member `value` located at `slot`.
     */
    function getAddressSlot(StoragePtr slot) internal pure returns (AddressSlot storage r) {
        assembly ("memory-safe") {
            r.slot := slot
        }
    }

    /**
     * @dev Converts a `AddressSlot` into an `StoragePtr`.
     */
    function asPtr(AddressSlot storage store) internal pure returns (StoragePtr ptr) {
        assembly ("memory-safe") {
            ptr := store.slot
        }
    }

    /**
     * @dev Returns a `BooleanSlot` with member `value` located at `slot`.
     */
    function getBooleanSlot(StoragePtr slot) internal pure returns (BooleanSlot storage r) {
        assembly ("memory-safe") {
            r.slot := slot
        }
    }

    /**
     * @dev Converts a `BooleanSlot` into an `StoragePtr`.
     */
    function asPtr(BooleanSlot storage store) internal pure returns (StoragePtr ptr) {
        assembly ("memory-safe") {
            ptr := store.slot
        }
    }

    /**
     * @dev Returns a `Bytes32Slot` with member `value` located at `slot`.
     */
    function getBytes32Slot(StoragePtr slot) internal pure returns (Bytes32Slot storage r) {
        assembly ("memory-safe") {
            r.slot := slot
        }
    }

    /**
     * @dev Converts a `Bytes32Slot` into an `StoragePtr`.
     */
    function asPtr(Bytes32Slot storage store) internal pure returns (StoragePtr ptr) {
        assembly ("memory-safe") {
            ptr := store.slot
        }
    }

    /**
     * @dev Returns a `Uint256Slot` with member `value` located at `slot`.
     */
    function getUint256Slot(StoragePtr slot) internal pure returns (Uint256Slot storage r) {
        assembly ("memory-safe") {
            r.slot := slot
        }
    }

    /**
     * @dev Converts a `Uint256Slot` into an `StoragePtr`.
     */
    function asPtr(Uint256Slot storage store) internal pure returns (StoragePtr ptr) {
        assembly ("memory-safe") {
            ptr := store.slot
        }
    }

    /**
     * @dev Returns a `Int256Slot` with member `value` located at `slot`.
     */
    function getInt256Slot(StoragePtr slot) internal pure returns (Int256Slot storage r) {
        assembly ("memory-safe") {
            r.slot := slot
        }
    }

    /**
     * @dev Converts a `Int256Slot` into an `StoragePtr`.
     */
    function asPtr(Int256Slot storage store) internal pure returns (StoragePtr ptr) {
        assembly ("memory-safe") {
            ptr := store.slot
        }
    }

    /**
     * @dev Returns an `StringSlot` representation of the string storage pointer `store`.
     */
    function getStringSlot(StoragePtr slot) internal pure returns (StringSlot storage r) {
        assembly ("memory-safe") {
            r.slot := slot
        }
    }

    /**
     * @dev Returns a `BytesSlot` with member `value` located at `slot`.
     */
    function getBytesSlot(StoragePtr slot) internal pure returns (BytesSlot storage r) {
        assembly ("memory-safe") {
            r.slot := slot
        }
    }

    /**
     * @dev Converts a `BytesSlot` into an `StoragePtr`.
     */
    function asPtr(BytesSlot storage store) internal pure returns (StoragePtr ptr) {
        assembly ("memory-safe") {
            ptr := store.slot
        }
    }
}
