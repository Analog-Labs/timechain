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
                    fail(2, 18, 0x626c61626c61206c6f68616e6e2061717569)
                }
            }

            // Touch the contract address and check if it exists
            {
                let codehash := extcodehash(calldataload(36))
                if iszero(codehash) {
                    // account doesn't exists
                    fail(3, 22, 0x6163636f756e7420646f65736e277420657869737473)
                }
                if eq(codehash, 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470) {
                    // not a contract
                    fail(4, 14, 0x6e6f74206120636f6e7472616374)
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
                        fail(5, 21, 0x74782064617461206f7574206f6620626f756e6473)
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
                        fail(2, 18, 0x696e73756666696369656e742066756e6473)
                    }
                }

                // resume gas metering
                resumeGasMetering()

                // prank(address,address)
                {
                    let callerMode := readCallers()

                    // User mode should be NONE
                    if gt(callerMode, 0) {
                        fail(6, 30, 0x706c732064697361626c65207072616e6b206f722062726f616463617374)
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
            function fail(status, size, message) {
                // Prefix message with "Internal Error: "
                message := shl(shl(3, sub(32, size)), message)
                let prefix := shl(128, 0x496e7465726e616c204572726f723a20)
                prefix, message := concatStr(prefix, 16, message)
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
                    fail(500, 18, 0x666f72676520766d206572726f72)
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
                    // vm.pauseGasMetering() failed
                    fail(0xd1a5b36f, 28, 0x766d2e70617573654761734d65746572696e672829206661696c6564)
                }
            }

            // Resumes gas metering
            // Ref: https://book.getfoundry.sh/cheatcodes/pause-gas-metering
            function resumeGasMetering() {
                let ptr := alloc(0)
                // 0x2bcd50e0 == bytes4(keccak256("resumeGasMetering()"))
                mstore(ptr, shl(224, 0x2bcd50e0))
                if callForgeVM(ptr, 4) {
                    // vm.resumeGasMetering() failed
                    fail(0x2bcd50e0, 29, 0x766d2e726573756d654761734d65746572696e672829206661696c6564)
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
                    // vm.readCallers() failed
                    fail(0x4ad0bac9, 23, 0x766d2e7265616443616c6c6572732829206661696c6564)
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
                    // vm.prank() failed
                    fail(0x47e50cce, 17, 0x766d2e7072616e6b2829206661696c6564)
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
                    // vm.startPrank() failed
                    fail(0x45b56078, 22, 0x766d2e73746172745072616e6b2829206661696c6564)
                }
            }

            // Stops an active prank started by startPrank
            // Ref: https://book.getfoundry.sh/cheatcodes/prank
            function stopPrank() {
                let ptr := alloc(0)
                // 0x90c5013b == bytes4(keccak256("stopPrank()"))
                mstore(ptr, shl(224, 0x90c5013b))
                if callForgeVM(ptr, 4) {
                    // vm.stopPrank() failed
                    fail(0x90c5013b, 21, 0x766d2e73746f705072616e6b2829206661696c6564)
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
                    // vm.deal(address,uint256) failed
                    fail(0xc88a5e6d, 31, 0x766d2e6465616c28616464726573732c75696e7432353629206661696c6564)
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
                    // vm.load(address,bytes32) failed
                    fail(0x667f9d70, 31, 0x766d2e6c6f616428616464726573732c6279746573333229206661696c6564)
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
                    fail(0x70ca10bb, 32, 0x766d2e73746f726528616464726573732c6279746573333229206661696c6564)
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
                    fail(0xf28dceb3, 30, 0x766d2e65787065637452657665727428737472696e6729206661696c6564)
                }
            }

            function allowCheatcodes(addr) {
                let ptr := alloc(0)
                // 0xea060291 == bytes4(keccak256("allowCheatcodes(address)"))
                mstore(ptr, shl(224, 0xea060291))
                mstore(add(ptr, 4), addr)
                if callForgeVM(ptr, 36) {
                    // vm.allowCheatcodes(address) fail
                    fail(0xea060291, 32, 0x766d2e616c6c6f774368656174636f646573286164647265737329206661696c)
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
