object "TestUtils" {
    // This is the constructor code of the contract.
    code {
        // Deploy the contract
        datacopy(0, dataoffset("runtime"), datasize("runtime"))
        return(0, datasize("runtime"))
    }

    object "runtime" {
        code {
            // Free memory pointer
            mstore(0x40, 0x80)

            // Pause gas metering
            pauseGasMetering()

            // Verify if the gas left is enough
            {
                let gasLimit := calldataload(68)
                let gasAvailable := gas()
                gasAvailable := saturatingSub(gasAvailable, 5000)
                gasAvailable := sub(gasAvailable, shr(6, gasAvailable))

                if lt(gasAvailable, gasLimit) {
                    fail(21, "insufficient gas left")
                }
            }

            // Touch the contract address and check if it exists
            {
                let codehash := extcodehash(calldataload(36))
                if iszero(codehash) {
                    fail(22, "account doesn't exists")
                }
                if eq(codehash, 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470) {
                    // not a contract
                    fail(14, "not a contract")
                }
            }

            {
                // Copy calldata to memory
                let ptr
                {
                    let offset := saturatingAdd(calldataload(100), 4)
                    let size := saturatingAdd(calldataload(offset), 32)

                    // Check if calldata is within bounds
                    if or(gt(saturatingAdd(offset, size), calldatasize()), lt(offset, 132)) {
                        fail(21, "tx data out of bounds")
                    }

                    ptr := alloc(alignValue(size))
                    calldatacopy(ptr, offset, size)
                }

                // Calculate base cost
                let baseCost := calculateBaseCost(add(ptr, 32), mload(ptr))

                // Check if the sender has funds for paying for the transaction fees
                {
                    let gasLimit := calldataload(68)
                    let sender := calldataload(4)
                    let senderBalance := balance(sender)
                    let weiCost := mul(add(baseCost, gasLimit), gasprice())

                    if lt(senderBalance, weiCost) {
                        // Log error
                        logStrAddrUintUint(
                            dataoffset("insufficient_funds"),
                            datasize("insufficient_funds"),
                            sender,
                            weiCost,
                            senderBalance
                        )
                        fail(18, "insufficient funds")
                    }
                }

                // resume gas metering
                resumeGasMetering()

                // prank(address,address)
                {
                    let callerMode := readCallers()

                    // User mode should be NONE
                    if gt(callerMode, 0) {
                        fail(26, "disable prank or broadcast")
                    }
                    let sender := calldataload(4)
                    prank(sender, sender)
                }

                // Execute call
                let success, gasUsed := verbatim_7i_2o(
                    hex"813f509594939291905a96f15a90910360699003",
                    calldataload(68), // gas limit
                    calldataload(36), // address
                    callvalue(), // value
                    add(ptr, 32), // arg offset
                    mload(ptr), // arg size
                    0,
                    0
                )
                let returnSize := returndatasize()
                mstore(0, gasUsed)
                mstore(32, baseCost)
                mstore(64, success)
                mstore(96, 128)
                mstore(128, returnSize)
                returndatacopy(160, 0, returnSize)

                if iszero(success) {
                    revert(160, returnSize)
                }

                return(0, add(160, returnSize))
            }

            function saturatingAdd(x, y) -> r {
                x := add(x, y)
                y := sub(0, gt(y, x))
                r := or(x, y)
            }

            function saturatingSub(x, y) -> r {
                r := mul(sub(x, y), gt(x, y))
            }

            // Align value to 256bit word
            function alignValue(v) -> r {
                r := shl(5, shr(5, add(v, 31)))
            }

            // Allocate memory
            function alloc(size) -> ptr {
                ptr := mload(0x40)
                mstore(0x40, add(ptr, size))
            }

            // Return an internal error
            function fail(size, message) {
                // Prefix message with "Internal Error: "
                let prefix
                prefix, message := concatStr("Internal Error: ", 16, message)
                size := add(size, 16)

                // Encode error data
                let ptr := alloc(132)
                mstore(ptr, shl(224, 0x08c379a0))
                mstore(add(ptr, 4), 32) // message offset
                mstore(add(ptr, 36), size) // message size
                mstore(add(ptr, 68), prefix)
                mstore(add(ptr, 100), message)

                // Resume gas metering before revert
                resumeGasMetering()

                revert(ptr, add(68, alignValue(size)))
            }

            // Concatenate two strings
            function concatStr(prefix, size, str) -> r0, r1 {
                let shift := shl(3, size)
                r0 := or(prefix, shr(shift, str))
                r1 := shl(sub(256, shift), str)
            }

            // Count non-zero bytes in a 256bit word in parallel
            // Reference: https://graphics.stanford.edu/~seander/bithacks.html#CountBitsSetParallel
            function countNonZeros(v) -> r {
                v := or(v, shr(4, v))
                v := or(v, shr(2, v))
                v := or(v, shr(1, v))
                v := and(
                    v,
                    0x0101010101010101010101010101010101010101010101010101010101010101
                )
                v := add(v, shr(128, v))
                v := add(v, shr(64, v))
                v := add(v, shr(32, v))
                v := add(v, shr(16, v))
                v := add(v, shr(8, v))
                r := and(v, 0xff)
            }

            // Calculate the tx base cost, defined as:
            // baseCost = 21000 + zeros * 4 + nonZeros * 16
            // Reference: https://eips.ethereum.org/EIPS/eip-2028
            function calculateBaseCost(ptr, size) -> r {
                let nonZeros := 0
                for {
                    let end := add(ptr, size)
                } lt(ptr, end) { ptr := add(ptr, 32) } {
                    nonZeros := add(nonZeros, countNonZeros(mload(ptr)))
                }

                let zeros := sub(size, nonZeros)
                r := add(mul(zeros, 4), mul(nonZeros, 16))
                r := add(r, 21000)
            }

            // Calls forge vm
            function callForgeVM(ptr, size) -> r {
                let success := call(
                    gas(), // call gas limit
                    0x7109709ECfa91a80626fF3989D68f67F5b1DD12D, // Forge VM address
                    0, // value in wei to transfer
                    ptr, // input memory pointer
                    size, // input size
                    0, // output memory pointer
                    0 // output size
                )

                // Copy result to memory
                size := returndatasize()
                returndatacopy(ptr, 0, size)

                // vm call reverted
                if iszero(success) {
                    consoleLog(ptr, size)
                    fail(18, "forge vm error")
                }
                r := size
            }

            // Pauses gas metering
            // Ref: https://book.getfoundry.sh/cheatcodes/pause-gas-metering
            function pauseGasMetering() {
                let ptr := alloc(0)
                // 0xd1a5b36f == bytes4(keccak256("pauseGasMetering()"))
                mstore(ptr, shl(224, 0xd1a5b36f))
                if callForgeVM(ptr, 4) {
                    fail(28, "vm.pauseGasMetering() failed")
                }
            }

            // Resumes gas metering
            // Ref: https://book.getfoundry.sh/cheatcodes/pause-gas-metering
            function resumeGasMetering() {
                let ptr := alloc(0)
                // 0x2bcd50e0 == bytes4(keccak256("resumeGasMetering()"))
                mstore(ptr, shl(224, 0x2bcd50e0))
                if callForgeVM(ptr, 4) {
                    fail(29, "vm.resumeGasMetering() failed")
                }
            }

            // Reads the current CallerMode
            // Ref: https://book.getfoundry.sh/cheatcodes/pause-gas-metering
            function readCallers() -> callerMode {
                let ptr := alloc(0)
                // 0x4ad0bac9 == bytes4(keccak256("readCallers()"))
                mstore(ptr, shl(224, 0x4ad0bac9))
                let r := callForgeVM(ptr, 4)
                if iszero(eq(r, 96)) {
                    fail(23, "vm.readCallers() failed")
                }
                callerMode := mload(ptr)
            }

            // Sets msg.sender and tx.origin to the specified address for the next call
            // Ref: https://book.getfoundry.sh/cheatcodes/prank
            function prank(sender, txOrigin) {
                let ptr := alloc(0)
                // 0x47e50cce == bytes4(keccak256("prank(address,address)"))
                mstore(ptr, shl(224, 0x47e50cce))
                mstore(add(ptr, 4), sender)
                mstore(add(ptr, 36), txOrigin)
                if callForgeVM(ptr, 68) {
                    fail(17, "vm.prank() failed")
                }
            }

            // Sets msg.sender for all subsequent calls until stopPrank is called.
            // Ref: https://book.getfoundry.sh/cheatcodes/prank
            function startPrank(sender, txOrigin) {
                let ptr := alloc(0)
                // 0x45b56078 == bytes4(keccak256("startPrank(address,address)"))
                mstore(ptr, shl(224, 0x45b56078))
                mstore(add(ptr, 4), sender)
                mstore(add(ptr, 36), txOrigin)
                if callForgeVM(ptr, 68) {
                    fail(22, "vm.startPrank() failed")
                }
            }

            // Stops an active prank started by startPrank
            // Ref: https://book.getfoundry.sh/cheatcodes/prank
            function stopPrank() {
                let ptr := alloc(0)
                // 0x90c5013b == bytes4(keccak256("stopPrank()"))
                mstore(ptr, shl(224, 0x90c5013b))
                if callForgeVM(ptr, 4) {
                    fail(21, "vm.stopPrank() failed")
                }
            }

            // Sets the balance of an address who to newBalance.
            // Ref: https://book.getfoundry.sh/cheatcodes/deal
            function deal(who, newBalance) {
                let ptr := alloc(0)
                // 0xc88a5e6d == bytes4(keccak256("deal(address,uint256)"))
                mstore(ptr, shl(224, 0xc88a5e6d))
                mstore(add(ptr, 4), who)
                mstore(add(ptr, 36), newBalance)
                if callForgeVM(ptr, 68) {
                    fail(31, "vm.deal(address,uint256) failed")
                }
            }

            // Loads the value from storage slot slot on account account.
            // Ref: https://book.getfoundry.sh/cheatcodes/load
            function vmLoad(account, slot) -> leet {
                let ptr := alloc(0)
                // 0x667f9d70 == bytes4(keccak256("load(address,bytes32)"))
                mstore(ptr, shl(224, 0x667f9d70))
                mstore(add(ptr, 4), account)
                mstore(add(ptr, 36), slot)
                if eq(callForgeVM(ptr, 68), 32) {
                    fail(31, "vm.load(address,bytes32) failed")
                }
                leet := mload(ptr)
            }

            // Loads the value from storage slot slot on account account.
            // Ref: https://book.getfoundry.sh/cheatcodes/load
            function vmStore(account, slot, value) {
                let ptr := alloc(0)
                // 0x70ca10bb == bytes4(keccak256("store(address,bytes32,bytes32)"))
                mstore(ptr, shl(224, 0x70ca10bb))
                mstore(add(ptr, 4), account)
                mstore(add(ptr, 36), slot)
                mstore(add(ptr, 68), value)
                if callForgeVM(ptr, 100) {
                    // vm.store(address,bytes32) failed
                    fail(32, "vm.store(address,bytes32) failed")
                }
            }

            function expectRevertStr(offset, size) {
                let ptr := alloc(0)
                // 0xf28dceb3 == bytes4(keccak256("expectRevert(bytes)"))
                mstore(ptr, shl(224, 0xf28dceb3))
                mstore(add(ptr, 4), 0x20) // offset
                mstore(add(ptr, 36), size) // size
                codecopy(add(ptr, 68), offset, size) // string
                size := alignValue(size)
                if callForgeVM(ptr, add(68, size)) {
                    // vm.expectRevert(string) failed
                    fail(30, "vm.expectRevert(string) failed")
                }
            }

            function allowCheatcodes(addr) {
                let ptr := alloc(0)
                // 0xea060291 == bytes4(keccak256("allowCheatcodes(address)"))
                mstore(ptr, shl(224, 0xea060291))
                mstore(add(ptr, 4), addr)
                if callForgeVM(ptr, 36) {
                    // vm.allowCheatcodes(address) fail
                    fail(31, "allowCheatcodes(address) failed")
                }
            }

            // log(string)
            function logEmit(offset, size) {
                let ptr := alloc(0)
                mstore(ptr, 0x20) // offset
                mstore(add(ptr, 32), size) // size
                codecopy(add(ptr, 64), offset, size) // string
                size := alignValue(size)
                log1(
                    ptr,
                    add(64, size),
                    0x41304facd9323d75b11bcdd609cb38effffdb05710f7caf0e9b16c6d9d709f50 // keccak256("log(string)")
                )
            }

            // log_named_uint(string,uint256)
            function log_named_uint(offset, size, value) {
                let ptr := alloc(0)
                mstore(ptr, 0x40) // offset
                mstore(add(ptr, 32), value) // value
                mstore(add(ptr, 64), size) // size
                codecopy(add(ptr, 96), offset, size) // string
                size := alignValue(size)
                log1(
                    ptr,
                    add(96, size),
                    0xb2de2fbe801a0df6c0cbddfd448ba3c41d48a040ca35c56c8196ef0fcae721a8 // keccak256("log_named_uint(string,uint256)")
                )
            }

            function consoleLog(payloadStart, payloadLength) {
                pop(staticcall(
                    gas(),
                    0x000000000000000000636F6e736F6c652e6c6f67, // console address
                    payloadStart,
                    payloadLength,
                    0,
                    0
                ))
            }

            // log(string)
            function logString(offset, size) {
                let ptr := alloc(0)
                // 0x41304fac == bytes4(keccak256("log(string)"))
                mstore(ptr, shl(224, 0x41304fac))
                mstore(add(ptr, 4), 0x20) // offset
                mstore(add(ptr, 36), size) // size
                codecopy(add(ptr, 68), offset, size) // string
                size := alignValue(size)
                consoleLog(ptr, add(100, size))
            }

            // log(string,uint256,uint256)
            function logStrUintUint(offset, size, v1, v2) {
                let ptr := alloc(0)
                // 0xca47c4eb == bytes4(keccak256("log(string,uint256,uint256)"))
                mstore(ptr, shl(224, 0xca47c4eb))
                mstore(add(ptr, 4), 0x60) // offset
                mstore(add(ptr, 36), v1) // v1
                mstore(add(ptr, 68), v2) // v2
                mstore(add(ptr, 100), size) // str size
                codecopy(add(ptr, 132), offset, size) // string
                size := alignValue(size)
                consoleLog(ptr, add(164, size))
            }

            // log(string,address,uint256,uint256)
            function logStrAddrUintUint(offset, size, addr0, v2, v3) {
                let ptr := alloc(0)
                // 0xf8f51b1e == bytes4(keccak256("log(string,address,uint256,uint256)"))
                mstore(ptr, shl(224, 0xf8f51b1e))
                mstore(add(ptr, 4), 0x80) // offset
                mstore(add(ptr, 36), addr0) // addr0
                mstore(add(ptr, 68), v2) // v2
                mstore(add(ptr, 100), v3) // v3
                mstore(add(ptr, 132), size) // str size
                codecopy(add(ptr, 164), offset, size) // string
                size := alignValue(size)
                consoleLog(ptr, add(196, size))
            }
        }

        data "insufficient_funds" "Error: account %s has no sufficient funds, require %s have %s"
    }
}
