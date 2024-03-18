pragma solidity ^0.8.24;

import "contracts/gateway.sol";
import "frost-evm/sol/Signer.sol";
import "forge-std/Test.sol";
import "forge-std/console.sol";

uint256 constant secret = 0x42;
uint256 constant nonce = 0x69;

interface TestUtils {
    function execute(address sender, address addr, uint256 gasLimit, bytes calldata data)
        external
        returns (uint256, uint256, bool, bytes memory);
}

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

contract GatewayTest is Test {
    Gateway gateway;
    Signer signer;

    // Receiver Contract, the will waste the exact amount of gas you sent to it in the data field
    IGmpReceiver receiver;

    // Receiver Contract, the will waste the exact amount of gas you sent to it in the data field
    TestUtils helperCtr;

    uint256 private constant EXECUTE_CALL_COST = 47_278;
    uint256 private constant SUBMIT_GAS_COST = 5539;
    uint16 private constant SRC_NETWORK_ID = 0;
    uint16 private constant DEST_NETWORK_ID = 69;
    uint256 private immutable GAS_LIMIT = (block.gaslimit / 5) * 4; // 80% of the block gas limit
    uint8 private constant GMP_STATUS_SUCCESS = 1;

    function setUp() public {
        // check block gas limit as gas left
        assertEq(block.gaslimit, 30_000_000);
        assertTrue(gasleft() >= 10_000_000);

        // Deploy the receiver contract
        // Obs: This is a special contract that wastes an exact amount of gas you send to it, helpful for testing GMP refunds and gas limits.
        // See the file `HelperContract.opcode` for more details.
        bytes memory bytecode = new bytes(96);
        assembly ("memory-safe") {
            let ptr := add(bytecode, 32)
            mstore(ptr, 0x7f60a4355a0360080180603b015b805a11600c57505a03604103565b5b5b5b5b)
            mstore(add(ptr, 32), 0x5b6000527f5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b0000000000000000000000)
            mstore(add(ptr, 64), 0x000000000060205260316000f300000000000000000000000000000000000000)
            let addr := create(0, ptr, 77)
            if iszero(addr) {
                mstore(ptr, shl(224, 0x08c379a0))
                mstore(add(ptr, 4), 0x20)
                mstore(add(ptr, 36), 0x1f)
                mstore(add(ptr, 68), 0x4661696c656420746f206465706c6f792048656c706572436f6e747261637400)
                revert(ptr, 100)
            }
            sstore(receiver.slot, addr)
        }
    }

    constructor() {
        signer = new Signer(secret);
        TssKey[] memory keys = new TssKey[](1);
        keys[0] = TssKey({yParity: signer.yParity() == 28 ? 1 : 0, xCoord: signer.xCoord()});
        gateway = new Gateway(DEST_NETWORK_ID, keys);

        // Deploy TestUtil.yul contract
        bytes memory bytecode =
            hex"61075c8061000c5f395ff3fe608060405261000c6104c6565b604435806100235a61138881119061138719010290565b8060061c9003106101645760243590813f801561015f577fc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a4701461015a57610073606435600401806004115f031790565b906100868235602001806020115f031790565b60848310368285018084115f0317111761015557601f8101601f19166100b0604051918201604052565b9283378151926100c4602084019485610396565b936004359283313a828801028082106101405750506100e1610544565b6100e96105c6565b61013b575f946100fa858796610649565b51923491813f509594939291905a96f15a90910360699003913d925f5260205280604052608060605281608052815f60a03e156101375760a0015ff35b60a0fd5b61032f565b610150925085603d61071f6106cd565b6102d4565b610276565b610220565b6101c1565b608460405180820160405262461bcd60e51b815260206004820152602260248201527f496e7465726e616c204572726f723a20626c61626c61206c6f68616e6e206171604482015261756960f01b60648201526101bf610544565bfd5b608460405180820160405262461bcd60e51b815260206004820152602660248201527f496e7465726e616c204572726f723a206163636f756e7420646f65736e27742060448201526565786973747360d01b60648201526101bf610544565b60646040516084810160405262461bcd60e51b815260206004820152601e60248201527f496e7465726e616c204572726f723a206e6f74206120636f6e7472616374000060448201525f828201526101bf610544565b608460405180820160405262461bcd60e51b815260206004820152602560248201527f496e7465726e616c204572726f723a2074782064617461206f7574206f6620626044820152646f756e647360d81b60648201526101bf610544565b608460405180820160405262461bcd60e51b815260206004820152602260248201527f496e7465726e616c204572726f723a20696e73756666696369656e742066756e604482015261647360f01b60648201526101bf610544565b608460405180820160405262461bcd60e51b815260206004820152602e60248201527f496e7465726e616c204572726f723a20706c732064697361626c65207072616e60448201526d1ac81bdc88189c9bd85918d85cdd60921b60648201526101bf610544565b5f908281015b8082106103b857505090816152089260041b910360021b010190565b909160209060ff84518060041c178060021c178060011c177f01010101010101010101010101010101010101010101010101010101010101016f010101010101010101010101010101018260801c169116018060401c0180841c018060101c018060081c01160192019061039c565b905f80918382737109709ecfa91a80626ff3989d68f67f5b1dd12d5af1903d91825f833e15610454575090565b5f9182916a636f6e736f6c652e6c6f675afa506040516084810160405262461bcd60e51b815260206004820152602260248201527f496e7465726e616c204572726f723a2000000000666f72676520766d2065727260448201526137b960f11b60648201526104c1610544565b608490fd5b6104dd600460405163d1a5b36f60e01b8152610427565b6104e357565b608460405180820160405262461bcd60e51b815260206004820152602c60248201527f496e7465726e616c204572726f723a20766d2e70617573654761734d6574657260448201526b1a5b99ca0a4819985a5b195960a21b60648201526101bf5b61055b600460405163015e6a8760e51b8152610427565b61056157565b6040516084810160405262461bcd60e51b815260206004820152602d60248201527f496e7465726e616c204572726f723a20766d2e726573756d654761734d65746560448201526c1c9a5b99ca0a4819985a5b1959609a1b60648201526104c1610544565b604051634ad0bac960e01b815260606105e0600483610427565b036105e9575190565b608460405180820160405262461bcd60e51b815260206004820152602760248201527f496e7465726e616c204572726f723a20766d2e7265616443616c6c65727328296044820152660819985a5b195960ca1b60648201526101bf610544565b60449061066d92604051916323f2866760e11b835260048301526024820152610427565b61067357565b608460405180820160405262461bcd60e51b815260206004820152602160248201527f496e7465726e616c204572726f723a20766d2e7072616e6b2829206661696c656044820152601960fa1b60648201526101bf610544565b81905f95869560405195637c7a8d8f60e11b87526080600488015260248701526044860152606485015281608485015260a4840139601f801991011660c401906a636f6e736f6c652e6c6f675afa5056fe4572726f723a206163636f756e7420257320686173206e6f2073756666696369656e742066756e64732c20726571756972652025732068617665202573";
        assembly ("memory-safe") {
            let ptr := add(bytecode, 32)
            let size := mload(bytecode)
            let addr := create(0, ptr, size)
            if iszero(addr) {
                mstore(ptr, shl(224, 0x08c379a0))
                mstore(add(ptr, 4), 0x20)
                mstore(add(ptr, 36), 0x1e)
                mstore(add(ptr, 68), 0x4661696c656420746f206465706c6f7920546573745574696c732e79756c0000)
                revert(ptr, 100)
            }
            sstore(helperCtr.slot, addr)
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
    function _executeCall(address dest, uint256 gasLimit, bytes memory data)
        private
        returns (uint256 gasUsed, bool success, bytes memory out)
    {
        assembly ("memory-safe") {
            {
                let zero := 0 // This is a dummy variable to guarantee a predictable stack layout
                let ptr := add(data, 32)
                let size := mload(data)
                let startGas := gas()
                success :=
                    call(
                        gasLimit, // call gas limit
                        dest, // dest address
                        zero, // value in wei to transfer
                        ptr, // input memory pointer
                        size, // input size
                        zero, // output memory pointer
                        zero // output size
                    )
                let endGas := gas()
                gasUsed := sub(startGas, endGas)
            }
            out := mload(0x40)
            let size := returndatasize()
            mstore(out, size)
            let ptr := add(out, 32)
            returndatacopy(ptr, 0, size)
            mstore(0x40, add(ptr, size))

            if iszero(success) { revert(ptr, size) }
        }
    }

    // Execute a contract call and calculate the acurrate execution gas cost
    function executeCall(address sender, address dest, uint256 gasLimit, bytes memory data)
        internal
        returns (uint256 executionCost, uint256 baseCost, bytes memory out)
    {
        bytes memory executeCallData = abi.encodeCall(TestUtils.execute, (sender, dest, gasLimit, data));
        bool success;
        uint256 status;
        bytes memory ptr;
        assembly ("memory-safe") {
            success :=
                delegatecall(
                    gas(), // call gas limit
                    sload(helperCtr.slot), // dest address
                    // 0, // value
                    add(32, executeCallData), // input memory pointer
                    mload(executeCallData), // input size
                    0, // output memory pointer
                    0 // output size
                )

            // Alloc memory
            ptr := mload(0x40)
            let size := returndatasize()
            mstore(ptr, size)
            ptr := add(ptr, 32)
            mstore(0x40, add(ptr, size))

            // Copy return data
            returndatacopy(ptr, 0, size)

            // If call reverted
            if iszero(success) {
                executionCost := 0
                baseCost := 0

                // Check if the call reverted due an internal error
                let signature := mul(mload(ptr), gt(size, 127))
                if eq(signature, 0xa04f7977a7361020e07a160885cb05178aa6d9ff17ec37e942914b4316b9352a) {
                    status := mload(add(ptr, 32))
                    out := add(ptr, 96)
                    revert(add(out, 32), mload(out))
                }

                // The call reverted, return the error message
                revert(ptr, size)
            }

            // If success, get the execution cost
            if success {
                if lt(size, 160) { revert(0, 0) }
                executionCost := mload(ptr)
                baseCost := mload(add(ptr, 32))
                status := mload(add(ptr, 64))
                out := add(ptr, 128)
            }
        }
        if (status > 1) {
            // Fail if the account doesn't have funds for paying for the tx fees.
            fail();
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
        (uint256 execution, uint256 base, bytes memory output) =
            executeCall(sender, address(gateway), gasLimit, abi.encodeCall(Gateway.execute, (signature, message)));
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
        address mockSender = address(0x0);
        vm.prank(mockSender);
        vm.expectRevert();
        gateway.deposit{value: 1}(0x0, 0);
    }

    function testDepositReducesSenderFunds() public {
        uint256 amount = 100 ether;
        address mockSender = address(0x0);
        vm.deal(mockSender, amount);
        uint256 balanceBefore = address(mockSender).balance;
        vm.prank(mockSender);
        gateway.deposit{value: amount}(0x0, 0);
        assertEq(balanceBefore - address(mockSender).balance, amount, "deposit failed to transfer amount from sender");
    }

    function testDepositIncreasesGatewayFunds() public {
        uint256 amount = 100 ether;
        address mockSender = address(0x0);
        address gatewayAddress = address(gateway);
        assert(gatewayAddress != mockSender);
        uint256 gatewayBalanceBefore = gatewayAddress.balance;
        vm.deal(mockSender, amount);
        vm.prank(mockSender);
        gateway.deposit{value: amount}(0x0, 0);
        assertEq(gatewayAddress.balance - gatewayBalanceBefore, amount, "deposit failed to transfer amount to gateway");
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
        executeCall(address(0), address(receiver), 100_000, testEncodedCall);
        (uint256 gasUsed,, bytes memory output) = executeCall(address(0), address(receiver), 100_000, testEncodedCall);
        assertEq(gasUsed, 1234);
        assertEq(output.length, 0);
    }

    function testDepositMapping() public {
        vm.txGasPrice(1);
        address mockSender = address(0x0);
        vm.deal(mockSender, GAS_LIMIT * 3);

        // GMP message gas used
        uint256 gmpGasUsed = 1_000;
        uint256 expectGasUsed = EXECUTE_CALL_COST + gmpGasUsed;

        // Deposit funds
        assertEq(gateway.depositOf(bytes32(bytes20(mockSender)), SRC_NETWORK_ID), 0);
        vm.prank(mockSender);
        gateway.deposit{value: expectGasUsed}(bytes32(bytes20(mockSender)), SRC_NETWORK_ID);
        assertEq(gateway.depositOf(bytes32(bytes20(mockSender)), SRC_NETWORK_ID), expectGasUsed);

        // Build and sign GMP message
        GmpMessage memory gmp = GmpMessage({
            source: bytes32(bytes20(mockSender)),
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
        uint256 beforeBalance = address(mockSender).balance;
        (uint8 status, bytes32 returned, uint256 gasUsed,) = executeGmp(sig, gmp, gasLimit, mockSender);
        uint256 afterBalance = address(mockSender).balance;
        assertEq(gasUsed, expectGasUsed, "unexpected gas used");
        assertEq(returned, expectResult, "unexpected GMP result");

        // Verify the gas refund
        assertEq((afterBalance - beforeBalance), gasUsed, "wrong refund amount");
        assertEq(gateway.depositOf(bytes32(bytes20(mockSender)), SRC_NETWORK_ID), 0);

        // Verify the GMP message status
        assertEq(status, GMP_STATUS_SUCCESS, "Unexpected GMP status");
        GmpInfo memory info = gateway.gmpInfo(keccak256(gateway.getGmpTypedHash(gmp)));
        assertEq(info.status, GMP_STATUS_SUCCESS, "GMP status stored doesn't match the returned status");
        assertEq(info.result, expectResult, "GMP result stored doesn't match the returned result");
    }

    function testExecuteRevertsWrongNetwork() public {
        vm.txGasPrice(1);
        uint256 amount = 10 ether;
        address mockSender = address(0x0);
        vm.deal(mockSender, amount * 2);

        gateway.deposit{value: amount}(0x0, 0);
        GmpMessage memory wrongNetwork = GmpMessage({
            source: 0x0,
            srcNetwork: 1,
            dest: address(0x0),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 1000,
            salt: 1,
            data: ""
        });
        Signature memory wrongNetworkSig = sign(wrongNetwork);
        vm.expectRevert(bytes("deposit below max refund"));
        executeGmp(wrongNetworkSig, wrongNetwork, 100_000, mockSender);
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
        vm.deal(address(0), 100_000_000_000_000);
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
        vm.deal(mockSender, 100_000_000_000_000_000);
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
        vm.deal(mockSender, 124252);
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
        vm.expectRevert("gas left below message.gasLimit");
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
        vm.expectRevert("message already executed");
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
            gasLimit: 10000,
            salt: 0,
            data: abi.encodePacked(uint256(100_000))
        });

        bytes32 id = keccak256(gateway.getGmpTypedHash(gmp));

        // Touch the gateway contract
        vm.prank(gmpSender);
        gateway.deposit{value: 1}(0x0, 0);
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
