// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/interfaces/IUpgradable.sol)

pragma solidity >=0.8.0;

interface IUpgradable {
    // The new implementation address is a not a contract
    error InvalidContract();
    // The supplied codehash does not match the new implementation codehash
    error InvalidCodeHash();

    // The implementation contract was upgraded
    event Upgraded(address indexed implementation);
}
