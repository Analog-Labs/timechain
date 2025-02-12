// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/utils/ERC1967.sol)

pragma solidity >=0.8.0;

/// @title Minimal implementation of ERC1967 storage slot
library ERC1967 {
    /**
     * @dev Storage slot with the address of the current implementation.
     * This is the keccak-256 hash of "eip1967.proxy.implementation" subtracted by 1.
     */
    // solhint-disable-next-line private-vars-leading-underscore
    bytes32 internal constant IMPLEMENTATION_SLOT = 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;

    /**
     * @dev Storage slot with the admin of the contract.
     * This is the keccak-256 hash of "eip1967.proxy.admin" subtracted by 1.
     */
    // solhint-disable-next-line private-vars-leading-underscore
    bytes32 internal constant ADMIN_SLOT = 0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103;

    /**
     * @dev Emitted when the implementation is upgraded.
     */
    event Upgraded(address indexed implementation);

    /**
     * @dev Emitted when the admin account has changed.
     */
    event AdminChanged(address previousAdmin, address newAdmin);

    /**
     * @dev The `implementation` of the proxy is invalid.
     */
    error ERC1967InvalidImplementation(address implementation);

    /**
     * @dev The `admin` of the proxy is invalid.
     */
    error ERC1967InvalidAdmin(address admin);

    /**
     * @dev Returns the current admin.
     *
     * TIP: To get this value clients can read directly from the storage slot shown below (specified by ERC-1967) using
     * the https://eth.wiki/json-rpc/API#eth_getstorageat[`eth_getStorageAt`] RPC call.
     * `0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103`
     */
    function getAdmin() internal view returns (address) {
        return _getAddressSlot(ADMIN_SLOT);
    }

    /**
     * @dev Stores a new address in the ERC-1967 admin slot.
     */
    function setAdmin(address newAdmin) internal {
        if (newAdmin == address(0)) {
            revert ERC1967InvalidAdmin(address(0));
        }
        emit AdminChanged(getAdmin(), newAdmin);
        _setAddressSlot(ADMIN_SLOT, newAdmin);
    }

    /**
     * @dev Returns the current implementation address.
     */
    function getImplementation() internal view returns (address) {
        return _getAddressSlot(IMPLEMENTATION_SLOT);
    }

    /**
     * @dev Stores a new address in the ERC-1967 implementation slot.
     */
    function setImplementation(address newImplementation) internal {
        if (newImplementation.code.length == 0) {
            revert ERC1967InvalidImplementation(newImplementation);
        }
        emit Upgraded(newImplementation);
        _setAddressSlot(IMPLEMENTATION_SLOT, newImplementation);
    }

    function _getAddressSlot(bytes32 slot) private view returns (address addr) {
        assembly {
            addr := sload(slot)
        }
    }

    function _setAddressSlot(bytes32 slot, address addr) private {
        assembly {
            sstore(slot, addr)
        }
    }
}
