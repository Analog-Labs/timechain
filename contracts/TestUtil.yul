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
                    failure(18, 0x626c61626c61206c6f68616e6e20617175690000000000000000000000000000)
                }
            }

            // Touch the contract address and check if it exists
            {
                let codehash := extcodehash(calldataload(36))
                if iszero(codehash) {
                    // account doesn't exists
                    failure(22, 0x6163636f756e7420646f65736e27742065786973747300000000000000000000)
                }
                if eq(codehash, 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470) {
                    // not a contract
                    failure(14, 0x6e6f74206120636f6e7472616374000000000000000000000000000000000000)
                }
            }

            {
                // Copy calldata to memory
                let ptr := mload(0x40)
                {
                    let offset := saturatingAdd(calldataload(100), 4)
                    let size := saturatingAdd(calldataload(offset), 32)

                    // Check if calldata is within bounds
                    if or(gt(saturatingAdd(offset, size), calldatasize()), lt(offset, 132)) {
                        failure(21, 0x74782064617461206f7574206f6620626f756e64730000000000000000000000)
                    }

                    calldatacopy(ptr, offset, size)
                    mstore(0x40, add(ptr, alignValue(size)))
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
                        fail()
                    }
                }

                // resume gas metering
                resumeGasMetering()

                // prank(address,address)
                {
                    let callerMode := readCallers()

                    switch callerMode
                    case 0 {
                        let sender := calldataload(4)
                        prank(sender, sender)
                    }
                    default {
                        failure(32, 0x766d2e7072616e6b28616464726573732c6164647265737329206661696c6564)
                    }
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

            function failure(size, message) {
                mstore(0, shl(224, 0x08c379a0))
                mstore(4, 32)
                mstore(36, size)
                mstore(68, message)
                revert(0, 0x64)
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

                // Revert
                if iszero(success) {
                    revert(ptr, size)
                }
                r := size
            }

            // Pauses gas metering
            // Ref: https://book.getfoundry.sh/cheatcodes/pause-gas-metering
            function pauseGasMetering() {
                let ptr := mload(0x40)
                // 0xd1a5b36f == bytes4(keccak256("pauseGasMetering()"))
                mstore(ptr, shl(224, 0xd1a5b36f))
                if callForgeVM(ptr, 4) {
                    // vm.pauseGasMetering() failed
                    failure(28, 0x766d2e70617573654761734d65746572696e672829206661696c656400000000)
                }
            }

            // Resumes gas metering
            // Ref: https://book.getfoundry.sh/cheatcodes/pause-gas-metering
            function resumeGasMetering() {
                let ptr := mload(0x40)
                // 0x2bcd50e0 == bytes4(keccak256("resumeGasMetering()"))
                mstore(ptr, shl(224, 0x2bcd50e0))
                if callForgeVM(ptr, 4) {
                    // vm.resumeGasMetering() failed
                    failure(29, 0x766d2e726573756d654761734d65746572696e672829206661696c6564000000)
                }
            }

            // Reads the current CallerMode
            // Ref: https://book.getfoundry.sh/cheatcodes/pause-gas-metering
            function readCallers() -> callerMode {
                let ptr := mload(0x40)
                // 0x4ad0bac9 == bytes4(keccak256("readCallers()"))
                mstore(ptr, shl(224, 0x4ad0bac9))
                let r := callForgeVM(ptr, 4)
                if iszero(eq(r, 96)) {
                    // vm.readCallers() failed
                    failure(23, 0x766d2e7265616443616c6c6572732829206661696c6564000000000000000000)
                }
                callerMode := mload(ptr)
            }

            // Sets msg.sender and tx.origin to the specified address for the next call
            // Ref: https://book.getfoundry.sh/cheatcodes/prank
            function prank(sender, txOrigin) {
                let ptr := mload(0x40)
                // 0x47e50cce == bytes4(keccak256("prank(address,address)"))
                mstore(ptr, shl(224, 0x47e50cce))
                mstore(add(ptr, 4), sender)
                mstore(add(ptr, 36), txOrigin)
                if callForgeVM(ptr, 68) {
                    // vm.prank() failed
                    failure(17, 0x766d2e7072616e6b2829206661696c6564000000000000000000000000000000)
                }
            }

            // Sets msg.sender for all subsequent calls until stopPrank is called.
            // Ref: https://book.getfoundry.sh/cheatcodes/prank
            function startPrank(sender, txOrigin) {
                let ptr := mload(0x40)
                // 0x45b56078 == bytes4(keccak256("startPrank(address,address)"))
                mstore(ptr, shl(224, 0x45b56078))
                mstore(add(ptr, 4), sender)
                mstore(add(ptr, 36), txOrigin)
                if callForgeVM(ptr, 68) {
                    // vm.startPrank() failed
                    failure(22, 0x766d2e73746172745072616e6b2829206661696c656400000000000000000000)
                }
            }

            // Stops an active prank started by startPrank
            // Ref: https://book.getfoundry.sh/cheatcodes/prank
            function stopPrank() {
                let ptr := mload(0x40)
                // 0x90c5013b == bytes4(keccak256("stopPrank()"))
                mstore(ptr, shl(224, 0x90c5013b))
                if callForgeVM(ptr, 4) {
                    // vm.stopPrank() failed
                    failure(21, 0x766d2e73746f705072616e6b2829206661696c65640000000000000000000000)
                }
            }

            // Sets the balance of an address who to newBalance.
            // Ref: https://book.getfoundry.sh/cheatcodes/deal
            function deal(who, newBalance) {
                let ptr := mload(0x40)
                // 0xc88a5e6d == bytes4(keccak256("deal(address,uint256)"))
                mstore(ptr, shl(224, 0xc88a5e6d))
                mstore(add(ptr, 4), who)
                mstore(add(ptr, 36), newBalance)
                if callForgeVM(ptr, 68) {
                    // vm.deal(address,uint256) failed
                    failure(31, 0x766d2e6465616c28616464726573732c75696e7432353629206661696c656400)
                }
            }

            // Loads the value from storage slot slot on account account.
            // Ref: https://book.getfoundry.sh/cheatcodes/load
            function vmLoad(account, slot) -> leet {
                let ptr := mload(0x40)
                // 0x667f9d70 == bytes4(keccak256("load(address,bytes32)"))
                mstore(ptr, shl(224, 0x667f9d70))
                mstore(add(ptr, 4), account)
                mstore(add(ptr, 36), slot)
                if eq(callForgeVM(ptr, 68), 32) {
                    // vm.load(address,bytes32) failed
                    failure(31, 0x766d2e6c6f616428616464726573732c6279746573333229206661696c656400)
                }
                leet := mload(ptr)
            }

            // Loads the value from storage slot slot on account account.
            // Ref: https://book.getfoundry.sh/cheatcodes/load
            function vmStore(account, slot, value) {
                let ptr := mload(0x40)
                // 0x70ca10bb == bytes4(keccak256("store(address,bytes32,bytes32)"))
                mstore(ptr, shl(224, 0x70ca10bb))
                mstore(add(ptr, 4), account)
                mstore(add(ptr, 36), slot)
                mstore(add(ptr, 68), value)
                if callForgeVM(ptr, 100) {
                    // vm.store(address,bytes32) failed
                    failure(32, 0x766d2e73746f726528616464726573732c6279746573333229206661696c6564)
                }
            }

            function expectRevertStr(offset, size) {
                let ptr := mload(0x40)
                // 0xf28dceb3 == bytes4(keccak256("expectRevert(bytes)"))
                mstore(ptr, shl(224, 0xf28dceb3))
                mstore(add(ptr, 4), 0x20) // offset
                mstore(add(ptr, 36), size) // size
                codecopy(add(ptr, 68), offset, size) // string
                size := alignValue(size)
                if callForgeVM(ptr, add(68, size)) {
                    // vm.expectRevert(string) failed
                    failure(30, 0x766d2e65787065637452657665727428737472696e6729206661696c65640000)
                }
            }

            function allowCheatcodes(addr) {
                let ptr := mload(0x40)
                // 0xea060291 == bytes4(keccak256("allowCheatcodes(address)"))
                mstore(ptr, shl(224, 0xea060291))
                mstore(add(ptr, 4), addr)
                if callForgeVM(ptr, 36) {
                    // vm.allowCheatcodes(address) fail
                    failure(32, 0x766d2e616c6c6f774368656174636f646573286164647265737329206661696c)
                }
            }

            function fail() {
                mstore(0, 0) // gas used
                mstore(32, 0) // base cost
                mstore(64, 2) // not enought funds
                mstore(96, 128) // offset
                mstore(128, 0) // return size
                return (0,160)
            }

            // log(string)
            function logEmit(offset, size) {
                let ptr := mload(0x40)
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
                let ptr := mload(0x40)
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
                let ptr := mload(0x40)
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
                let ptr := mload(0x40)
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
                let ptr := mload(0x40)
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

            //
            function hasHEVMContext() -> r {
                r := extcodesize(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D)
            }

            function assertGe(a, b) {
                if lt(a, b) {
                    logEmit(dataoffset("assertGe"), datasize("assertGe"))
                    log_named_uint(dataoffset("assertGe_a"), datasize("assertGe_a"), a)
                    log_named_uint(dataoffset("assertGe_b"), datasize("assertGe_b"), b)
                    fail()
                }
            }

        }

        data "assertGe" "Error: a >= b not satisfied [uint]"
        data "assertGe_a" "  Value a"
        data "assertGe_b" "  Value b"
        data "insufficient_funds" "Error: account %s has no sufficient funds, require %s have %s"
    }
}
