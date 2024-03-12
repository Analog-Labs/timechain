pragma solidity ^0.8.20;

import "contracts/gateway.sol";
import "frost-evm/sol/Signer.sol";
import "forge-std/Test.sol";
// import "forge-std/console.sol";

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

contract GatewayTest is Test {
    Gateway gateway;
    Signer signer;

    // Receiver Contract, the will waste the exact amount of gas you sent to it in the data field
    IGmpReceiver receiver;
    uint256 EXECUTE_CALL_COST;

    function setUp() public {
        // check block gas limit as gas left
        assertEq(block.gaslimit, 30_000_000);
        assertTrue(gasleft() >= 10_000_000);

        // Deploy helper contract
        // Obs: This is a special contract that wastes an exact amount of gas you send to it, helpful for testing GMP refunds and gas limits.
        // See the file `HelperContract.opcode` for more details.
        bytes memory bytecode = new bytes(96);
        assembly ("memory-safe") {
            let ptr := add(bytecode, 32)
            mstore(ptr, 0x7f60a4355a0360080180603b015b805a11600c57505a03604103565b5b5b5b5b)
            mstore(add(ptr, 32), 0x5b6000527f5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b0000000000000000000000)
            mstore(add(ptr, 64), 0x000000000060205260316000f300000000000000000000000000000000000000)
            let addr := create(0, ptr, 77)
            if iszero(addr) { revert(0, 0) }
            sstore(receiver.slot, addr)
        }
    }

    constructor() {
        signer = new Signer(secret);
        TssKey[] memory keys = new TssKey[](1);
        keys[0] = TssKey({yParity: signer.yParity() == 28 ? 1 : 0, xCoord: signer.xCoord()});
        gateway = new Gateway(69, keys);
        EXECUTE_CALL_COST = 47_304;
    }

    function sign(GmpMessage memory gmp) internal view returns (Signature memory) {
        uint256 hash = uint256(keccak256(gateway.getGmpTypedHash(gmp)));
        // uint256 hash = uint256(bytes32(0xf99063356a8a2dd3bcac17a8fe7ef581ab43865e519f872185ea85216d53404b));
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
        returns (uint256 execution, uint256 baseCost, bytes memory out)
    {
        // Execute
        vm.startBroadcast(sender);
        (uint256 executionCost, bool success, bytes memory result) = _executeCall(dest, gasLimit, data);
        vm.stopBroadcast();
        executionCost -= 128;

        // Compute the base tx cost (21k + 4 * zeros + 16 * nonZeros)
        uint256 zeros = countBytes(data, 0);
        uint256 nonZeros = data.length - zeros;
        uint256 inputCost = (nonZeros * 16) + (zeros * 4);

        out = result;
        execution = executionCost;
        baseCost = inputCost + 21_000;

        // Revert on failure
        if (!success) {
            assembly ("memory-safe") {}
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

    uint16 private constant SRC_NETWORK_ID = 0;
    uint16 private constant DEST_NETWORK_ID = 3;
    uint256 private constant GAS_LIMIT = 100_000_000; // 100_008_677
    uint8 private constant GMP_STATUS_SUCCESS = 1;

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
        uint256 expectGasUsed = 47_304 + gmpGasUsed;

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
        uint256 gasLimit = expectGasUsed + 2151;
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
            destNetwork: 0x0,
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
            destNetwork: 0x0,
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
            destNetwork: 0x0,
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
            destNetwork: 0x0,
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
            destNetwork: 0x0,
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
            destNetwork: 0x0,
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
}
