pragma solidity ^0.8.24;

import "contracts/gateway.sol";
import "frost-evm/sol/Signer.sol";
// import "forge-std/Test.sol";
import "forge-std/console.sol";
import "forge-std/StdJson.sol";
import "./TestUtils.sol";

uint256 constant secret = 0x42;
uint256 constant nonce = 0x69;

interface TransactionExecutor {
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
    using stdJson for string;
    using TestUtils for address;

    struct GmpResult {
        uint8 status;
        bytes32 result;
        uint256 executionCost;
        uint256 baseCost;
    }

    Gateway gateway;
    Signer signer;

    // Receiver Contract, the will waste the exact amount of gas you sent to it in the data field
    IGmpReceiver receiver;

    // Receiver Contract, the will waste the exact amount of gas you sent to it in the data field
    TransactionExecutor helperCtr;

    uint256 private constant EXECUTE_CALL_COST = 47_278;
    uint256 private constant SUBMIT_GAS_COST = 5539 + 27;
    uint16 private constant SRC_NETWORK_ID = 0;
    uint16 private constant DEST_NETWORK_ID = 69;
    uint256 private immutable GAS_LIMIT = (block.gaslimit / 5) * 4; // 80% of the block gas limit
    uint8 private constant GMP_STATUS_SUCCESS = 1;

    function setUp() public {
        // check block gas limit as gas left
        assertEq(block.gaslimit, 30_000_000);
        assertTrue(gasleft() >= 10_000_000);

        // Deploy the TransactionExecutor
        // See the file `TransactionExecutor.yul` for more details.
        {
            string memory root = vm.projectRoot();
            string memory path = string.concat(root, "/out/TransactionExecutor.yul/TransactionExecutor.json");
            bytes memory bytecode = vm.readFile(path).readBytes(".bytecode.object");
            helperCtr = TransactionExecutor(TestUtils.deployContract(bytecode));
            vm.allowCheatcodes(address(helperCtr));
        }

        // Deploy the Receiver contract
        // Obs: This is a special contract that wastes an exact amount of gas you send to it, helpful for testing GMP refunds and gas limits.
        // See the file `HelperContract.opcode` for more details.
        {
            bytes memory bytecode =
                hex"6031600d60003960316000f3fe60a4355a0360080180603b015b805a11600c57505a03604103565b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b00";
            receiver = IGmpReceiver(TestUtils.deployContract(bytecode));
        }
    }

    constructor() {
        signer = new Signer(secret);
        TssKey[] memory keys = new TssKey[](1);
        keys[0] = TssKey({yParity: signer.yParity() == 28 ? 1 : 0, xCoord: signer.xCoord()});
        gateway = new Gateway(DEST_NETWORK_ID, keys);
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

    // Execute a contract call and calculate the acurrate execution gas cost
    function executeCall(address sender, address dest, uint256 gasLimit, bytes memory data)
        internal
        returns (uint256 executionCost, uint256 baseCost, bytes memory out)
    {
        uint256 txGasCost = TestUtils.calculateBaseCost(data) + gasLimit;
        uint256 txGasPrice = txGasCost * tx.gasprice;
        uint256 senderBalance = sender.balance;
        require(senderBalance >= txGasPrice, "sender balance is not enough to pay for the fees");

        // Pay tx fees
        senderBalance -= txGasPrice;
        vm.deal(sender, senderBalance);
        require(sender.balance == senderBalance, "unexpected sender balance");

        bytes memory encodedCall = abi.encodeCall(TransactionExecutor.execute, (sender, dest, gasLimit, data));
        (bool success, bytes memory result) = TestUtils.delegateCall(address(helperCtr), encodedCall);
        if (success) {
            (executionCost, baseCost,, out) = abi.decode(result, (uint256, uint256, uint256, bytes));
        } else {
            executionCost = 0;
            baseCost = 0;
            assembly ("memory-safe") {
                revert(add(result, 32), mload(result))
            }
        }

        // Refund the difference between the gas limit and the execution cost
        uint256 gasDifference = gasLimit - executionCost;
        if (gasDifference > 0) {
            senderBalance = sender.balance;
            senderBalance += gasDifference * tx.gasprice;
            vm.deal(sender, senderBalance);
        }
    }

    // Allows you to define the gas limit for the GMP call, also retrieve a more accurate gas usage
    // by executing the GMP message.
    function executeGmp(
        Signature memory signature, // coordinate x, nonce, e, s
        GmpMessage memory message,
        uint256 gasLimit,
        address sender
    ) internal returns (GmpResult memory) {
        (uint256 executionCost, uint256 baseCost, bytes memory output) =
            executeCall(sender, address(gateway), gasLimit, abi.encodeCall(Gateway.execute, (signature, message)));

        if (output.length != 64) {
            return GmpResult({status: 0, result: bytes32(0), executionCost: 0, baseCost: 0});
        }
        require(output.length == 64, "unexpected gateway.execute output length");
        (uint8 status, bytes32 result) = abi.decode(output, (uint8, bytes32));
        return GmpResult({status: status, result: result, executionCost: executionCost, baseCost: baseCost});
    }

    function testDepositRevertsOutOfFunds() public {
        address mockSender = TestUtils.createTestAccount(0);
        vm.expectRevert();
        vm.prank(mockSender);
        gateway.deposit{value: 1}(mockSender.source(), 0);
    }

    function testDepositReducesSenderFunds() public {
        uint256 amount = 100 ether;
        address sender = TestUtils.createTestAccount(amount);
        uint256 balanceBefore = address(sender).balance;
        vm.prank(sender);
        gateway.deposit{value: amount}(sender.source(), 0);
        uint256 balanceAfter = address(sender).balance;
        assertEq(balanceAfter, balanceBefore - amount, "deposit failed to transfer amount from sender");
    }

    function testDepositIncreasesGatewayFunds() public {
        uint256 amount = 100 ether;
        address sender = TestUtils.createTestAccount(amount);
        address gatewayAddress = address(gateway);
        assert(gatewayAddress != sender);
        uint256 gatewayBalanceBefore = gatewayAddress.balance;
        vm.prank(sender);
        gateway.deposit{value: amount}(sender.source(), 0);
        uint256 gatewayBalanceAfter = gatewayAddress.balance;
        assertEq(gatewayBalanceAfter, gatewayBalanceBefore + amount, "deposit failed to transfer amount to gateway");
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
        vm.prank(sender);
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
        uint256 beforeBalance = address(sender).balance;
        GmpResult memory result = executeGmp(sig, gmp, expectGasUsed + 2160, sender);
        uint256 afterBalance = address(sender).balance;
        assertEq(result.executionCost, expectGasUsed, "unexpected gas used");
        assertEq(result.result, bytes32(0), "unexpected GMP result");
        // Verify the gas refund
        assertEq(afterBalance, (beforeBalance - result.baseCost), "wrong refund amount");
        assertEq(gateway.depositOf(sender.source(), SRC_NETWORK_ID), 0);

        // Verify the GMP message status
        assertEq(result.status, GMP_STATUS_SUCCESS, "Unexpected GMP status");
        GmpInfo memory info = gateway.gmpInfo(keccak256(gateway.getGmpTypedHash(gmp)));
        assertEq(info.status, GMP_STATUS_SUCCESS, "GMP status stored doesn't match the returned status");
        assertEq(info.result, bytes32(0), "GMP result stored doesn't match the returned result");
    }

    function testExecuteRevertsWrongNetwork() public {
        vm.txGasPrice(1);
        uint256 amount = 10 ether;
        address sender = TestUtils.createTestAccount(amount * 2);

        gateway.deposit{value: amount}(sender.source(), 0);
        GmpMessage memory wrongNetwork = GmpMessage({
            source: sender.source(),
            srcNetwork: 1,
            dest: address(receiver),
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
        address sender = TestUtils.createTestAccount(amount * 2);
        gateway.deposit{value: amount}(sender.source(), 0);
        GmpMessage memory wrongSource = GmpMessage({
            source: bytes32(uint256(0x1)),
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 1000,
            salt: 1,
            data: ""
        });
        Signature memory wrongSourceSig = sign(wrongSource);
        vm.expectRevert(bytes("deposit below max refund"));
        executeGmp(wrongSourceSig, wrongSource, 100_000, sender);
    }

    function testExecuteRevertsWithoutDeposit() public {
        vm.txGasPrice(1);
        address sender = TestUtils.createTestAccount(100 ether);
        GmpMessage memory gmp = GmpMessage({
            source: sender.source(),
            srcNetwork: 0,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 10_000,
            salt: 1,
            data: abi.encode(uint256(10_000))
        });
        Signature memory sig = sign(gmp);
        assertEq(gateway.depositOf(sender.source(), 0), 0);
        vm.expectRevert("deposit below max refund");
        executeGmp(sig, gmp, 100_000, sender);
    }

    function testExecuteRevertsBelowDeposit() public {
        vm.txGasPrice(1);
        uint256 gmpGasLimit = 10_000;
        uint256 insufficientDeposit = EXECUTE_CALL_COST + gmpGasLimit - 1;
        address sender = TestUtils.createTestAccount(100 ether);
        gateway.deposit{value: insufficientDeposit}(sender.source(), SRC_NETWORK_ID);
        GmpMessage memory gmp = GmpMessage({
            source: sender.source(),
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: gmpGasLimit,
            salt: 1,
            data: abi.encode(uint256(10_000))
        });
        Signature memory sig = sign(gmp);
        vm.expectRevert("deposit below max refund");
        executeGmp(sig, gmp, 100_000, sender);

        // Now deposit one extra wei amd ot should be enough
        gateway.deposit{value: 1}(sender.source(), SRC_NETWORK_ID);
        GmpResult memory result = executeGmp(sig, gmp, 100_000, sender);
        assertEq(result.executionCost, EXECUTE_CALL_COST + gmp.gasLimit);
        assertEq(result.status, GMP_STATUS_SUCCESS);
    }

    function testExecuteRevertsBelowGasLimit() public {
        vm.txGasPrice(1);
        uint256 gasLimit = 1_000_000;
        address sender = TestUtils.createTestAccount(100 ether);
        gateway.deposit{value: 20 ether}(sender.source(), SRC_NETWORK_ID);
        GmpMessage memory gmp = GmpMessage({
            source: sender.source(),
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 1_000_000, // require 1 million
            salt: 1,
            data: abi.encode(uint256(1_000_000))
        });
        Signature memory sig = sign(gmp);
        vm.expectRevert("gas left below message.gasLimit");
        executeGmp(sig, gmp, 100_000, sender); // give 100 thousand
    }

    function testExecuteRevertsAlreadyExecuted() public {
        vm.txGasPrice(1);
        uint256 amount = 100 ether;
        address sender = TestUtils.createTestAccount(amount * 2);
        gateway.deposit{value: amount}(sender.source(), SRC_NETWORK_ID);
        GmpMessage memory gmp = GmpMessage({
            source: sender.source(),
            srcNetwork: SRC_NETWORK_ID,
            dest: address(receiver),
            destNetwork: DEST_NETWORK_ID,
            gasLimit: 1000,
            salt: 1,
            data: abi.encode(uint256(1000))
        });
        Signature memory sig = sign(gmp);
        GmpResult memory result = executeGmp(sig, gmp, 100_000, sender);
        assertEq(result.status, GMP_STATUS_SUCCESS);
        vm.expectRevert("message already executed");
        executeGmp(sig, gmp, 100_000, sender);
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
