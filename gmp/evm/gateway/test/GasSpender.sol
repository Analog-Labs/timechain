// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/utils/GasSpender.sol)

pragma solidity >=0.8.0;

import {IGmpReceiver} from "gmp/IGmpReceiver.sol";

contract GasSpender is IGmpReceiver {
    // Following a contract implements the IGmpReceiver interface and wastes the exact amount of gas you send to it in the payload field.
    //  OFFSET  OPCODE
    //    0x00  0x5a      GAS
    //    0x01  0x6002    PUSH1 0x02
    //    0x03  0x01      ADD
    //    0x04  0x80      DUP1
    //    0x05  0x3d      RETURNDATASIZE
    //    0x06  0x52      MSTORE
    //    0x07  0x3d      RETURNDATASIZE
    //    0x08  0x6020    PUSH1 0x20
    //    0x0a  0x91      SWAP2
    //    0x0b  0x6084    PUSH1 0x84
    //    0x0d  0x35      CALLDATALOAD -- Load the payload offset from the calldata
    //    0x0e  0x6024    PUSH1 0x24
    //    0x10  0x01      ADD
    //    0x11  0x35      CALLDATALOAD -- Load the gasToWaste from the payload
    //    0x12  0x14      EQ
    //    0x13  0x6018    PUSH1 0x18
    // ,=<0x15  0x57      JUMPI
    // |  0x16  0x5b      JUMPDEST
    // |  0x17  0xfd      REVERT       -- Reverts if the gas left is less than the gas to waste.
    // |=>0x18  0x5b      JUMPDEST     -- Waste 22 gas each on each iteration
    // |  0x19  0x6036    PUSH1 0x36
    // |  0x1b  0x5a      GAS
    // |  0x1c  0x11      GT
    // |  0x1d  0x6018    PUSH1 0x18
    // `=<0x1f  0x57      JUMPI
    //    0x20  0x5a      GAS
    //    0x21  0x6049    PUSH1 0x49
    //    0x23  0x03      SUB
    // ,=<0x24  0x56      JUMP        -- Jumps depending on how much gas is left
    // |=>0x25  0x5b      JUMPDEST
    // |=>0x26  0x5b      JUMPDEST
    // |=>0x27  0x5b      JUMPDEST
    // |=>0x28  0x5b      JUMPDEST
    // |=>0x29  0x5b      JUMPDEST
    // |=>0x2a  0x5b      JUMPDEST
    // |=>0x2b  0x5b      JUMPDEST
    // |=>0x2c  0x5b      JUMPDEST
    // |=>0x2d  0x5b      JUMPDEST
    // |=>0x2e  0x5b      JUMPDEST
    // |=>0x2f  0x5b      JUMPDEST
    // |=>0x30  0x5b      JUMPDEST
    // |=>0x31  0x5b      JUMPDEST
    // |=>0x32  0x5b      JUMPDEST
    // |=>0x33  0x5b      JUMPDEST
    // |=>0x34  0x5b      JUMPDEST
    // |=>0x35  0x5b      JUMPDEST
    // |=>0x36  0x5b      JUMPDEST
    // |=>0x37  0x5b      JUMPDEST
    // |=>0x38  0x5b      JUMPDEST
    // |=>0x39  0x5b      JUMPDEST
    // `=>0x3a  0x5b      JUMPDEST
    //    0x3b  0xf3      RETURN
    bytes private constant BYTECODE =
        hex"5a600201803d523d60209160843560240135146018575bfd5b60365a116018575a604903565b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5bf3";

    constructor() payable {
        bytes memory bytecode = BYTECODE;
        assembly {
            return(add(bytecode, 0x20), mload(bytecode))
        }
    }

    function onGmpReceived(bytes32, uint128, bytes32, uint64, bytes calldata payload)
        external
        payable
        returns (bytes32)
    {
        unchecked {
            // OBS: This is just an example on how this contract works, the actual code is implemented directly in
            // low level EVM, as defined in the `BYTECODE` constant.
            uint256 initialGas = gasleft() + 2;
            uint256 gasToWaste = abi.decode(payload, (uint256));
            require(initialGas > gasToWaste);
            uint256 finalGas = initialGas - gasToWaste;
            while (gasleft() > finalGas) {}
            return bytes32(initialGas);
        }
    }
}
