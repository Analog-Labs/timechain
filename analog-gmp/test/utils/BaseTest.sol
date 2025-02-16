// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/utils/BaseTest.sol)

pragma solidity >=0.8.0;

import {IUniversalFactory} from "../../lib/universal-factory/src/IUniversalFactory.sol";
import {FactoryUtils} from "../../lib/universal-factory/src/FactoryUtils.sol";
import {Interpreter} from "../../lib/evm-interpreter/src/Interpreter.sol";
import {Test, console, Vm} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";

abstract contract BaseTest is Test {
    using FactoryUtils for IUniversalFactory;

    /**
     * @dev Universal Factory used to deploy contracts at deterministic addresses.
     * see: https://github.com/Analog-Labs/Universal-factory
     */
    IUniversalFactory internal constant FACTORY = IUniversalFactory(0x0000000000001C4Bf962dF86e38F0c10c7972C6E);

    /**
     * @dev EVM Interpreter, used to extract the `type(Gateway).runtimeCode` from the `type(Gateway).creationCode`.
     * see: https://github.com/analog-Labs/evm-interpreter
     */
    address internal constant EVM_INTERPRETER = 0x0000000000001e3F4F615cd5e20c681Cf7d85e8D;

    /**
     * @dev CREATE2 Salt used to deploy the `findBytes` contracts at deterministic addresses.
     */
    bytes32 private constant CREATE2_SALT = bytes32(uint256(1234));

    /**
     * @dev Address who must deploy the `UniversalFactory` contract, to guarantee the same address on all networks.
     */
    address internal constant FACTORY_DEPLOYER = 0x908064dE91a32edaC91393FEc3308E6624b85941;

    /**
     * @dev The `findBytes` contract bytecode, used to find byte sequences in a given bytecode.
     */
    bytes private constant FIND_BYTES =
        hex"602e80600a5f395ff3fe60403610602a573d35601f5b6001018035821881361102600b576020813614602a5790033d5260203df35b3d3dfd";

    /**
     * @dev CODEHASH from the `findBytes` contract bytecode, used to compute the final CREATE2 address..
     */
    bytes32 private constant FIND_BYTES_CODEHASH = keccak256(FIND_BYTES);

    /**
     * @dev All byte32 sequences in the form `0x7E7E7E7E7E7E...` will be replaced by the `INLINE_BYTECODE`.
     */
    bytes32 private constant INLINE_BYTECODE = 0x6000823f505a96949290959391f15a607b019091036800000000000000000052;

    constructor() {
        // Initialize the Universal Factory
        if (address(FACTORY).code.length == 0) {
            bytes memory creationCode = vm.getCode("./lib/universal-factory/abi/UniversalFactory.json");
            vm.deal(FACTORY_DEPLOYER, 100 ether);
            vm.prank(FACTORY_DEPLOYER, FACTORY_DEPLOYER);
            address factory;
            assembly {
                factory := create(0, add(creationCode, 32), mload(creationCode))
            }
            require(factory == address(FACTORY), "Factory address mismatch");
        }

        // Initialize the EVM Interpreter
        if (EVM_INTERPRETER.code.length == 0) {
            bytes memory creationCode = vm.getCode("Interpreter.sol");
            address interpreter;
            assembly {
                interpreter := create(0, add(creationCode, 32), mload(creationCode))
            }
            assertTrue(interpreter != address(0), "interpreter creation failed");
            assertGt(interpreter.code.length, 0, "interpreter code length mismatch");
            vm.etch(EVM_INTERPRETER, interpreter.code);
        }

        // Deploy the find bytes contract
        address findBytes = FACTORY.computeCreate2Address(CREATE2_SALT, FIND_BYTES_CODEHASH);

        // Initialize using the EVM interpreter
        if (findBytes.code.length == 0) {
            assertEq(FACTORY.create2(CREATE2_SALT, FIND_BYTES), findBytes, "find bytes creation failed");
            assembly {
                // Copy code to memory
                codecopy(0, 0, codesize())

                // Execute constructor using the EVM interpreter
                let success := delegatecall(gas(), EVM_INTERPRETER, 0, add(codesize(), 0x20), 0, 0)

                // Copy result to memory
                returndatacopy(0x20, 0, returndatasize())
                if iszero(success) { revert(0x20, returndatasize()) }

                // COPY TAG to memory
                let tag := mul(0x7E, 0x0101010101010101010101010101010101010101010101010101010101010101)
                mstore(0x00, tag)

                // Find the `0x7E7E7E...` tag in the bytecode
                let size := returndatasize()
                if iszero(staticcall(gas(), findBytes, 0x00, add(size, 0x20), 0x00, 0x20)) { revert(0, 0) }

                // Replace the `0x7E7E7E...` by the `INLINE_BYTECODE`
                let offset := add(mload(0x00), 0x20)
                mstore(add(offset, 1), 0x5B)
                mstore(offset, INLINE_BYTECODE)

                // Replace remaining occurences of `0x7E7E7E...` by the `INLINE_BYTECODE`

                for { let end := add(size, 0x20) } lt(offset, end) {} {
                    let backup := mload(offset)
                    mstore(offset, tag)
                    success := staticcall(gas(), findBytes, offset, sub(add(size, 0x20), offset), 0x00, 0x20)
                    mstore(offset, backup)
                    if iszero(success) { break }
                    offset := add(mload(0x00), 0x20)
                    mstore(add(offset, 1), 0x5B)
                    mstore(offset, INLINE_BYTECODE)
                }

                return(0x20, size)
            }
        }
    }
}
