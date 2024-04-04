pragma solidity ^0.8.24;

import "contracts/gateway.sol";
import "frost-evm/sol/Signer.sol";
import "forge-std/Test.sol";
import "forge-std/Vm.sol";
import "contracts/TestUtils.sol";

uint256 constant secret = 0x42;
uint256 constant nonce = 0x69;

contract SigUtilsTest is Test {
    function testPayload() public {
        SigUtils utils = new SigUtils(69, address(0));
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 42,
            dest: address(0x0),
            destNetwork: 69,
            gasLimit: 0,
            salt: 0,
            data: ""
        });
        bytes memory payload = utils.getGmpTypedHash(gmp);
        assertEq(
            payload,
            hex"19013e3afdf794f679fcbf97eba49dbe6b67cec6c7d029f1ad9a5e1a8ffefa8db2724ed044f24764343e77b5677d43585d5d6f1b7618eeddf59280858c68350af1cd"
        );
    }
}

contract GatewayBase is Test {
    using TestUtils for address;

    Gateway internal gateway;
    Signer internal signer;

    // Receiver Contract, the will waste the exact amount of gas you sent to it in the data field
    IGmpReceiver internal receiver;

    uint256 private constant EXECUTE_CALL_COST = 47_324;
    uint256 private constant SUBMIT_GAS_COST = 5539;
    uint16 private constant SRC_NETWORK_ID = 0;
    uint16 internal constant DEST_NETWORK_ID = 69;
    // uint256 private immutable GAS_LIMIT = (block.gaslimit / 5) * 4; // 80% of the block gas limit
    uint8 private constant GMP_STATUS_SUCCESS = 1;

    constructor() {
        signer = new Signer(secret);
        TssKey[] memory keys = new TssKey[](1);
        keys[0] = TssKey({yParity: signer.yParity() == 28 ? 1 : 0, xCoord: signer.xCoord()});
        gateway = new Gateway(DEST_NETWORK_ID, keys);
    }

    function setUp() public {
        // check block gas limit as gas left
        assertEq(block.gaslimit, 30_000_000);
        assertTrue(gasleft() >= 10_000_000);

        // Obs: This is a special contract that wastes an exact amount of gas you send to it, helpful for testing GMP refunds and gas limits.
        // See the file `HelperContract.opcode` for more details.
        {
            bytes memory bytecode =
                hex"6031600d60003960316000f3fe60a4355a0360080180603b015b805a11600c57505a03604103565b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b00";
            receiver = IGmpReceiver(TestUtils.deployContract(bytecode));
        }
    }

    function sign(GmpMessage memory gmp) internal view returns (Signature memory) {
        uint256 hash = uint256(keccak256(gateway.getGmpTypedHash(gmp)));
        (uint256 e, uint256 s) = signer.signPrehashed(hash, nonce);
        return Signature({xCoord: signer.xCoord(), e: e, s: s});
    }

    // Count the number of occurrences of a byte in a bytes array
    function countBytes(bytes memory input, uint8 haystack) internal pure returns (uint256 zeros) {
        assembly ("memory-safe") {
            zeros := 0
            let ptr := add(input, 32)
            let size := mload(input)

            let val
            for { let i := 0 } gt(size, i) { i := add(i, 1) } {
                let pos := mod(i, 32)
                if iszero(pos) { val := mload(add(ptr, i)) }
                zeros := add(zeros, eq(byte(pos, val), haystack))
            }
        }
    }

    // Workaround for set the tx.gasLimit, currently is not possible to define the gaslimit in foundry
    // Reference: https://github.com/foundry-rs/foundry/issues/2224
    function _call(address addr, uint256 gasLimit, bytes memory data)
        private
        returns (uint256 gasUsed, bool success, bytes memory out)
    {
        assertTrue(gasleft() > (gasLimit + 5000), "insufficient gas");
        assertTrue(addr.code.length > 0, "Not a contract address");
        uint256 gasAfter;
        assembly ("memory-safe") {
            let gasBefore := gas()
            success :=
                call(
                    0x7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E7E, // gas limit
                    addr, // addr
                    gasLimit, // value
                    add(data, 32),
                    mload(data),
                    0,
                    0
                )
            gasAfter := gas()
            gasAfter := sub(gasBefore, gasAfter)
            gasUsed := mload(mload(0x40))

            out := mload(0x40)
            let size := returndatasize()
            mstore(out, size)
            let ptr := add(out, 32)
            returndatacopy(ptr, 0, size)
            mstore(0x40, add(ptr, size))
        }
    }

    // Execute a contract call and calculate the acurrate execution gas cost
    function executeCall(address sender, address dest, uint256 gasLimit, bytes memory data)
        internal
        returns (uint256 executionCost, uint256 baseCost, bytes memory out)
    {
        // Compute the base tx cost (21k + 4 * zeros + 16 * nonZeros)
        {
            uint256 zeros = countBytes(data, 0);
            uint256 nonZeros = data.length - zeros;
            uint256 inputCost = (nonZeros * 16) + (zeros * 4);
            baseCost = inputCost + 21_000;
        }

        // Execute
        (VmSafe.CallerMode callerMode,,) = vm.readCallers();
        if (callerMode == VmSafe.CallerMode.None) {
            vm.prank(sender, sender);
        }
        bool success;
        (executionCost, success, out) = _call(dest, gasLimit, data);
        assembly {
            if iszero(success) { revert(add(out, 32), mload(out)) }
        }
    }

    // Allows you to define the gas limit for the GMP call, also retrieve a more accurate gas usage
    // by executing the GMP message.
    function executeGmp(
        Signature memory signature, // coordinate x, nonce, e, s
        GmpMessage memory message,
        uint256 gasLimit,
        address sender
    ) internal returns (uint8 status, bytes32 result, uint256 executionCost, uint256 baseCost) {
        bytes memory encodedCall = abi.encodeCall(Gateway.execute, (signature, message));
        (uint256 execution, uint256 base, bytes memory output) =
            executeCall(sender, address(gateway), gasLimit, encodedCall);
        executionCost = execution;
        baseCost = base;
        if (output.length == 64) {
            assembly {
                let ptr := add(output, 32)
                status := mload(ptr)
                result := mload(add(ptr, 32))
            }
        }
    }

    function testDepositRevertsOutOfFunds() public {
        address sender = TestUtils.createTestAccount(0);
        vm.prank(sender, sender);
        vm.expectRevert();
        gateway.deposit{value: 1}(sender.source(), 0);
    }

    function testDepositReducesSenderFunds() public {
        uint256 amount = 100 ether;
        address sender = TestUtils.createTestAccount(amount);
        uint256 balanceBefore = address(sender).balance;
        vm.prank(sender, sender);
        gateway.deposit{value: amount}(sender.source(), 0);
        uint256 balanceAfter = address(sender).balance;
        uint256 expectedBalance = balanceBefore - amount;
        assertEq(balanceAfter, expectedBalance, "deposit failed to transfer amount from sender");
    }

    function testDepositIncreasesGatewayFunds() public {
        uint256 amount = 100 ether;
        address sender = TestUtils.createTestAccount(amount);
        address gatewayAddress = address(gateway);
        assert(gatewayAddress != sender);
        uint256 balanceBefore = gatewayAddress.balance;
        vm.prank(sender, sender);
        gateway.deposit{value: amount}(sender.source(), 0);
        uint256 balanceAfter = gatewayAddress.balance;
        uint256 expectedBalance = balanceBefore + amount;
        assertEq(balanceAfter, expectedBalance, "deposit failed to transfer amount to gateway");
    }

    function testReceiver() public {
        bytes memory testEncodedCall = abi.encodeCall(
            IGmpReceiver.onGmpReceived,
            (
                0x0000000000000000000000000000000000000000000000000000000000000000,
                1,
                0x0000000000000000000000000000000000000000000000000000000000000000,
                abi.encode(uint256(1234))
            )
        );
        // Calling the receiver contract directly to make the address warm
        address sender = TestUtils.createTestAccount(10 ether);
        executeCall(sender, address(receiver), 100_000, testEncodedCall);
        (uint256 gasUsed,, bytes memory output) = executeCall(sender, address(receiver), 100_000, testEncodedCall);
        assertEq(gasUsed, 1234);
        assertEq(output.length, 0);
    }

    function testDepositMapping() public {
        vm.txGasPrice(1);
        address sender = TestUtils.createTestAccount(100 ether);

        // GMP message gas used
        uint256 gmpGasUsed = 1_000;
        uint256 expectGasUsed = EXECUTE_CALL_COST + gmpGasUsed;

        // Deposit funds
        assertEq(gateway.depositOf(sender.source(), SRC_NETWORK_ID), 0);
        vm.prank(sender, sender);
        gateway.deposit{value: expectGasUsed}(sender.source(), SRC_NETWORK_ID);
        assertEq(gateway.depositOf(sender.source(), SRC_NETWORK_ID), expectGasUsed);

        // Build and sign GMP message
        GmpMessage memory gmp = GmpMessage({
            source: sender.source(),
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 10000,
            salt: 1,
            data: abi.encode(gmpGasUsed)
        });
        Signature memory sig = sign(gmp);

        // Execute GMP message
        bytes32 expectResult = bytes32(0);
        uint256 gasLimit = expectGasUsed + 2160;
        uint256 beforeBalance = sender.balance;
        (uint8 status, bytes32 returned, uint256 gasUsed,) = executeGmp(sig, gmp, gasLimit, sender);
        uint256 afterBalance = sender.balance;
        assertEq(gasUsed, expectGasUsed, "unexpected gas used");
        assertEq(returned, expectResult, "unexpected GMP result");

        // Verify the gas refund
        assertEq((afterBalance - beforeBalance), gasUsed, "wrong refund amount");
        assertEq(gateway.depositOf(sender.source(), SRC_NETWORK_ID), 0);

        // Verify the GMP message status
        assertEq(status, GMP_STATUS_SUCCESS, "Unexpected GMP status");
        GmpInfo memory info = gateway.gmpInfo(keccak256(gateway.getGmpTypedHash(gmp)));
        assertEq(info.status, GMP_STATUS_SUCCESS, "GMP status stored doesn't match the returned status");
        assertEq(info.result, expectResult, "GMP result stored doesn't match the returned result");
    }

    function testExecuteRevertsWrongNetwork() public {
        vm.txGasPrice(1);
        uint256 amount = 10 ether;
        address sender = TestUtils.createTestAccount(amount * 2);

        gateway.deposit{value: amount}(sender.source(), 1234);
        GmpMessage memory wrongNetwork = GmpMessage({
            source: sender.source(),
            srcNetwork: 1,
            dest: address(0x0),
            destNetwork: 1234,
            gasLimit: 1000,
            salt: 1,
            data: ""
        });
        Signature memory wrongNetworkSig = sign(wrongNetwork);
        vm.expectRevert("invalid gmp network");
        executeGmp(wrongNetworkSig, wrongNetwork, 10_000, sender);
    }

    function testExecuteRevertsWrongSource() public {
        vm.txGasPrice(1);
        uint256 amount = 10 ether;
        address mockSender = address(0x0);
        vm.deal(mockSender, amount * 2);
        gateway.deposit{value: amount}(0x0, 0);
        GmpMessage memory wrongSource = GmpMessage({
            source: bytes32(uint256(0x1)),
            srcNetwork: 0,
            dest: address(0x0),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 1000,
            salt: 1,
            data: ""
        });
        Signature memory wrongSourceSig = sign(wrongSource);
        vm.expectRevert(bytes("deposit below max refund"));
        executeGmp(wrongSourceSig, wrongSource, 100_000, mockSender);
    }

    function testExecuteRevertsWithoutDeposit() public {
        vm.txGasPrice(1);
        GmpMessage memory gmp = GmpMessage({
            source: bytes32(0),
            srcNetwork: 0,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 1_000_000,
            salt: 1,
            data: abi.encode(uint256(1_000_000))
        });
        Signature memory sig = sign(gmp);
        assertEq(gateway.depositOf(bytes32(0), 0), 0);
        vm.expectRevert("deposit below max refund");
        executeGmp(sig, gmp, 1_500_000, address(0));
    }

    function testExecuteRevertsBelowDeposit() public {
        vm.txGasPrice(1);
        uint256 insufficientDeposit = EXECUTE_CALL_COST - 1;
        address mockSender = address(0x0);
        vm.deal(mockSender, insufficientDeposit);
        gateway.deposit{value: insufficientDeposit}(0x0, 0);
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 10000,
            salt: 1,
            data: abi.encode(uint256(10_000))
        });
        Signature memory sig = sign(gmp);
        vm.expectRevert("deposit below max refund");
        executeGmp(sig, gmp, 100_000, mockSender);
    }

    function testExecuteRevertsBelowGasLimit() public {
        vm.txGasPrice(1);
        uint256 gasLimit = 100000;
        uint256 insufficientDeposit = gasLimit * tx.gasprice;
        address mockSender = address(0x0);
        vm.deal(mockSender, insufficientDeposit);
        gateway.deposit{value: insufficientDeposit}(0x0, 0);
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: gasLimit,
            salt: 1,
            data: abi.encode(uint256(100_000))
        });
        Signature memory sig = sign(gmp);
        vm.expectRevert(bytes("gas left below message.gasLimit"));
        executeGmp(sig, gmp, 100_000, mockSender);
    }

    function testExecuteRevertsAlreadyExecuted() public {
        vm.txGasPrice(1);
        uint256 amount = 100 ether;
        address mockSender = address(0x0);
        vm.deal(mockSender, amount * 2);
        gateway.deposit{value: amount}(0x0, 0);
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 1000,
            salt: 1,
            data: abi.encode(uint256(1000))
        });
        Signature memory sig = sign(gmp);
        (uint8 status,,,) = executeGmp(sig, gmp, 100_000, mockSender);
        assertEq(status, GMP_STATUS_SUCCESS);
        vm.expectRevert(bytes("message already executed"));
        executeGmp(sig, gmp, 100_000, mockSender);
    }

    function testSubmitGmpMessage() public {
        vm.txGasPrice(1);
        address gmpSender = address(0x86E4Dc95c7FBdBf52e33D563BbDB00823894C287);
        vm.deal(gmpSender, 1_000_000_000_000_000_000);
        GmpMessage memory gmp = GmpMessage({
            source: bytes32(uint256(uint160(gmpSender))),
            srcNetwork: DEST_NETWORK_ID,
            dest: address(receiver),
            destNetwork: SRC_NETWORK_ID,
            gasLimit: 100000,
            salt: 0,
            // data: ""
            data: abi.encodePacked(uint256(100_000))
        });
        bytes32 id = keccak256(gateway.getGmpTypedHash(gmp));

        // Touch the gateway contract
        vm.prank(gmpSender, gmpSender);
        gateway.deposit{value: 1}(0x0, 0);
        {
            bytes memory encodedCall =
                abi.encodeCall(Gateway.submitMessage, (gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data));
            vm.expectRevert();
            executeCall(gmpSender, address(gateway), 1000, encodedCall);
        }
        assertEq(gateway.prevMessageHash(), bytes32(uint256(2 ** 256 - 1)), "WROONNGG");

        // Expect event
        vm.expectEmit(true, true, true, true);
        emit IGateway.GmpCreated(id, gmp.source, gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.salt, gmp.data);

        // Submit GMP message
        bytes memory encodedCall =
            abi.encodeCall(Gateway.submitMessage, (gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data));
        (uint256 execution, uint256 base, bytes memory output) =
            executeCall(gmpSender, address(gateway), 100_000, encodedCall);
        assertEq(output.length, 0, "unexpected gateway.submitMessage output");

        // Verify the gas cost
        uint256 expectedCost = SUBMIT_GAS_COST + 2800 + 351;
        assertEq(execution, expectedCost, "unexpected execution gas cost");

        // Now the second GMP message should have the salt equals to previous gmp hash
        gmp.salt = uint256(id);
        id = keccak256(gateway.getGmpTypedHash(gmp));

        // Expect event
        vm.expectEmit(true, true, true, true);
        emit IGateway.GmpCreated(id, gmp.source, gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.salt, gmp.data);

        // Submit GMP message
        encodedCall = abi.encodeCall(Gateway.submitMessage, (gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data));
        (execution, base, output) = executeCall(gmpSender, address(gateway), 100_000, encodedCall);
        assertEq(output.length, 0, "unexpected gateway.submitMessage output");

        // Verify the gas cost
        expectedCost = SUBMIT_GAS_COST + 351;
        assertEq(execution, expectedCost, "unexpected execution gas cost");

        // Now the second GMP message should have the salt equals to previous gmp hash
        gmp.salt = uint256(id);
        id = keccak256(gateway.getGmpTypedHash(gmp));

        // Expect event
        vm.expectEmit(true, true, true, true);
        emit IGateway.GmpCreated(id, gmp.source, gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.salt, gmp.data);

        // Submit GMP message
        encodedCall = abi.encodeCall(Gateway.submitMessage, (gmp.dest, gmp.destNetwork, gmp.gasLimit, gmp.data));
        (execution, base, output) = executeCall(gmpSender, address(gateway), 100_000, encodedCall);
        assertEq(output.length, 0, "unexpected gateway.submitMessage output");

        // Verify the gas cost
        expectedCost = SUBMIT_GAS_COST + 351;
        assertEq(execution, expectedCost, "unexpected execution gas cost");
    }
}

/**
 * @dev Workaround to fix Forge the gas report.
 *
 * Due to limitations in forge, the gas cost reported is misleading:
 * - https://github.com/foundry-rs/foundry/issues/6578
 * - https://github.com/foundry-rs/foundry/issues/6910
 *
 * The contract is a workaround that fixes it by inject an arbitrary code into the `GatewayBase`,
 * it replaces the constant `0x7E7E7E7E7E7E...` defined in the `_call` function by the `INLINE_BYTECODE`.
 * This guarantees us to precisely compute the execution gas cost.
 *
 * This workaround is necessary while solidity doesn't support verbatim in inline assembly code.
 * - https://github.com/ethereum/solidity/issues/12067
 *
 * @author Lohann Ferreira
 */
contract GatewayTest is GatewayBase {
    /**
     * @dev Bytecode that does an acurrate gas measurement of a call, it is equivalent to:
     * ```solidity
     * uint256 gasBefore = gasleft();
     * contract.call{gas: gasLimit}(data);
     * uint256 gasAfter = gasleft();
     * uint256 gasUsed = gasBefore - gasAfter - OVERHEAD;
     * assembly {
     *    mstore(mload(0x40), gasUsed)
     * }
     * ```
     * Solidity is a black box, is not possible to reliably calculate the `OVERHEAD` cost, creating a lot of
     * uncertainty in the gas measurements. `Yul` have the same issue once we don't control the EVM stack.
     * This code workaround this by doing the gas measurement right before and after execute the CALL opcode.
     */
    bytes32 private constant INLINE_BYTECODE = 0x6a0000000000000000000000919594939291905a96f15a606901909103604051;

    constructor() payable {
        // In solidity the child's constructor are executed before the parent's constructor,
        // so once this contract extends `GatewayBase`, it's constructor is executed first.

        // Copy `GatewayBase` runtime code into memory.
        bytes memory runtimeCode = type(GatewayBase).runtimeCode;

        // Replaces the first occurence of `0x7E7E..` in the runtime code by the `INLINE_BYTECODE`
        assembly ("memory-safe") {
            let size := mload(runtimeCode)
            let i := add(runtimeCode, 32)

            // Efficient Algorithm to find 32 consecutive repeated bytes in a byte sequence
            for {
                let chunk := 1
                let end := add(i, size)
            } gt(chunk, 0) { i := add(i, chunk) } {
                // Transform all `0x7E` bytes into `0xFF`
                // 0x81 ^ 0x7E == 0xFF
                // Also transform all other bytes in something different than `0xFF`
                chunk := xor(mload(i), 0x8181818181818181818181818181818181818181818181818181818181818181)

                // Find the right most unset bit, which is equivalent to find the
                // right most byte different than `0x7E`.
                // ex: (0x12345678FFFFFF + 1) & (~0x12345678FFFFFF) == 0x00000001000000
                chunk := and(add(chunk, 1), not(chunk))

                // Round down to the closest multiple of 256
                // Ex: 2 ** 18 become 2 ** 16
                chunk := div(chunk, mod(chunk, 0xff))

                // Find the number of leading bytes different than `0x7E`.
                // Rationale:
                // Multiplying a number by a power of 2 is the same as shifting the bits to the left
                // 1337 * (2 ** 16) == 1337 << 16
                // Once the chunk is a multiple of 256 it always shift entire bytes, we use this to
                // select a specific byte in a byte sequence.
                chunk := shr(248, mul(0x201f1e1d1c1b1a191817161514131211100f0e0d0c0b0a090807060504030201, chunk))

                // Stop the loop if we go out of bounds
                chunk := mul(chunk, lt(i, end))
            }

            // Check if we found the 32 byte constant `7E7E7E...`
            if not(xor(mload(i), 0x8181818181818181818181818181818181818181818181818181818181818181)) {
                let ptr := mload(0x40)
                mstore(ptr, shl(224, 0x08c379a0))
                mstore(add(ptr, 4), 32) // message offset
                mstore(add(ptr, 36), 29) // message size
                mstore(add(ptr, 68), "Failed to inject the bytecode")
                revert(ptr, 100)
            }

            // Replace the runtime code with the injected bytecode
            mstore(add(i, 1), 0x52)
            mstore(i, INLINE_BYTECODE)

            // Return the modified runtime code
            return(add(runtimeCode, 32), mload(runtimeCode))
        }
    }
}
