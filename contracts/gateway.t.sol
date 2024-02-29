pragma solidity ^0.8.20;

import "contracts/gateway.sol";
import "frost-evm/sol/Signer.sol";
import "forge-std/Test.sol";

uint256 constant secret = 0x42;
uint256 constant nonce = 0x69;

contract GatewayTest is Test {
    Gateway gateway;
    Signer signer;
    uint256 FOUNDRY_GAS_LIMIT;
    uint256 EXECUTE_CALL_COST;

    constructor() {
        signer = new Signer(secret);
        TssKey[] memory keys = new TssKey[](1);
        keys[0] = TssKey({yParity: signer.yParity() == 28 ? 1 : 0, xCoord: signer.xCoord()});
        gateway = new Gateway(69, keys);
        FOUNDRY_GAS_LIMIT = 9223372036854775807;
        EXECUTE_CALL_COST = 41153;
    }

    function sign(GmpMessage memory gmp) internal view returns (Signature memory) {
        uint256 hash = uint256(keccak256(gateway.getGmpTypedHash(gmp)));
        (uint256 e, uint256 s) = signer.signPrehashed(hash, nonce);
        return Signature({xCoord: signer.xCoord(), e: e, s: s});
    }

    function testPayload() public {
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 42,
            dest: address(0x0),
            destNetwork: 69,
            gasLimit: 0,
            salt: 0,
            data: ""
        });
        bytes memory payload = gateway.getGmpTypedHash(gmp);
        assertEq(payload, hex"19013e3afdf794f679fcbf97eba49dbe6b67cec6c7d029f1ad9a5e1a8ffefa8db2724ed044f24764343e77b5677d43585d5d6f1b7618eeddf59280858c68350af1cd");
    }

    function testDepositRevertsOutOfFunds() public {
        address mockSender = address(0x0);
        vm.startPrank(mockSender);
        vm.expectRevert();
        gateway.deposit{value: 1}(0x0, 0);
        vm.stopPrank();
    }

    function testDepositReducesSenderFunds() public {
        uint256 amount = 100 ether;
        address mockSender = address(0x0);
        vm.deal(mockSender, amount);
        uint256 balanceBefore = address(mockSender).balance;
        vm.startPrank(mockSender);
        gateway.deposit{value: amount}(0x0, 0);
        assertEq(balanceBefore - address(mockSender).balance, amount, "deposit failed to transfer amount from sender");
        vm.stopPrank();
    }

    function testDepositIncreasesGatewayFunds() public {
        uint256 amount = 100 ether;
        address mockSender = address(0x0);
        address gatewayAddress = address(gateway);
        assert(gatewayAddress != mockSender);
        uint256 gatewayBalanceBefore = gatewayAddress.balance;
        vm.deal(mockSender, amount);
        vm.startPrank(mockSender);
        gateway.deposit{value: amount}(0x0, 0);
        assertEq(gatewayAddress.balance - gatewayBalanceBefore, amount, "deposit failed to transfer amount to gateway");
        vm.stopPrank();
    }

    function testDepositMapping() public {
        vm.txGasPrice(1);
        address mockSender = address(0x0);
        vm.deal(mockSender, FOUNDRY_GAS_LIMIT * 3);
        vm.startPrank(mockSender);
        gateway.deposit{value: FOUNDRY_GAS_LIMIT + EXECUTE_CALL_COST - 1}(0x0, 0);
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(0x0),
            destNetwork: 0x0,
            gasLimit: FOUNDRY_GAS_LIMIT,
            salt: 1,
            data: ""
        });
        Signature memory sig = sign(gmp);
        (uint8 status,) = gateway.execute(sig, gmp);
        uint8 GMP_STATUS_SUCCESS = 1;
        assertEq(status, GMP_STATUS_SUCCESS);
        GmpMessage memory gmp2 = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(0x1),
            destNetwork: 0x0,
            gasLimit: FOUNDRY_GAS_LIMIT,
            salt: 1,
            data: ""
        });
        Signature memory sig2 = sign(gmp2);
        vm.expectRevert(bytes("deposit below max refund"));
        gateway.execute(sig2, gmp2);
        gateway.deposit{value: 1}(0x0, 0);
        vm.expectRevert(bytes("deposit below max refund"));
        gateway.execute(sig2, gmp2);
        gateway.deposit{value: 1}(0x0, 0);
        gateway.execute(sig2, gmp2);
        vm.stopPrank();
    }

    function testExecuteRevertsWrongNetwork() public {
        vm.txGasPrice(1);
        uint256 amount = 10 ether;
        address mockSender = address(0x0);
        vm.deal(mockSender, amount * 2);
        vm.startPrank(mockSender);
        gateway.deposit{value: amount}(0x0, 0);
        GmpMessage memory wrongNetwork = GmpMessage({
            source: 0x0,
            srcNetwork: 1,
            dest: address(0x0),
            destNetwork: 0x0,
            gasLimit: FOUNDRY_GAS_LIMIT,
            salt: 1,
            data: ""
        });
        Signature memory wrongNetworkSig = sign(wrongNetwork);
        vm.expectRevert(bytes("deposit below max refund"));
        gateway.execute(wrongNetworkSig, wrongNetwork);
        vm.stopPrank();
    }

    function testExecuteRevertsWrongSource() public {
        vm.txGasPrice(1);
        uint256 amount = 10 ether;
        address mockSender = address(0x0);
        vm.deal(mockSender, amount * 2);
        vm.startPrank(mockSender);
        gateway.deposit{value: amount}(0x0, 0);
        GmpMessage memory wrongSource = GmpMessage({
            source: bytes32(uint256(0x1)),
            srcNetwork: 0,
            dest: address(0x0),
            destNetwork: 0x0,
            gasLimit: FOUNDRY_GAS_LIMIT,
            salt: 1,
            data: ""
        });
        Signature memory wrongSourceSig = sign(wrongSource);
        vm.expectRevert(bytes("deposit below max refund"));
        gateway.execute(wrongSourceSig, wrongSource);
        vm.stopPrank();
    }

    function testExecuteRevertsWithoutDeposit() public {
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(0x0),
            destNetwork: 0x0,
            gasLimit: FOUNDRY_GAS_LIMIT,
            salt: 1,
            data: ""
        });
        Signature memory sig = sign(gmp);
        vm.expectRevert(bytes("deposit below max refund"));
        gateway.execute(sig, gmp);
    }

    function testExecuteRevertsBelowDeposit() public {
        vm.txGasPrice(1);
        uint256 insufficientDeposit = (FOUNDRY_GAS_LIMIT * tx.gasprice) - 1;
        address mockSender = address(0x0);
        vm.deal(mockSender, insufficientDeposit);
        vm.startPrank(mockSender);
        gateway.deposit{value: insufficientDeposit}(0x0, 0);
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(0x0),
            destNetwork: 0x0,
            gasLimit: FOUNDRY_GAS_LIMIT,
            salt: 1,
            data: ""
        });
        Signature memory sig = sign(gmp);
        vm.expectRevert(bytes("deposit below max refund"));
        gateway.execute(sig, gmp);
        vm.stopPrank();
    }

    function testExecuteRevertsBelowGasLimit() public {
        vm.txGasPrice(1);
        uint256 gasLimit = 100000;
        uint256 insufficientDeposit = gasLimit * tx.gasprice;
        address mockSender = address(0x0);
        vm.deal(mockSender, insufficientDeposit);
        vm.startPrank(mockSender);
        gateway.deposit{value: insufficientDeposit}(0x0, 0);
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(0x0),
            destNetwork: 0x0,
            gasLimit: gasLimit,
            salt: 1,
            data: ""
        });
        Signature memory sig = sign(gmp);
        vm.expectRevert(bytes("gas left below message.gasLimit"));
        gateway.execute(sig, gmp);
        vm.stopPrank();
    }

    function testExecuteRevertsAlreadyExecuted() public {
        vm.txGasPrice(1);
        uint256 amount = 100 ether;
        address mockSender = address(0x0);
        vm.deal(mockSender, amount * 2);
        vm.startPrank(mockSender, mockSender);
        gateway.deposit{value: amount}(0x0, 0);
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(0x0),
            destNetwork: 0x0,
            gasLimit: FOUNDRY_GAS_LIMIT,
            salt: 1,
            data: ""
        });
        Signature memory sig = sign(gmp);
        (uint8 status,) = gateway.execute(sig, gmp);
        uint8 GMP_STATUS_SUCCESS = 1;
        assertEq(status, GMP_STATUS_SUCCESS);
        vm.expectRevert(bytes("message already executed"));
        gateway.execute(sig, gmp);
        vm.stopPrank();
    }

    function testExecuteReimbursement() public {
        vm.txGasPrice(1);
        uint256 amount = 100 ether;
        address mockSender = address(0x0);
        address gatewayAddress = address(gateway);
        assert(gatewayAddress != mockSender);
        vm.deal(mockSender, amount * 2);
        vm.startPrank(mockSender, mockSender);
        gateway.deposit{value: amount}(0x0, 0);
        assertEq(mockSender.balance, amount);
        assertEq(gatewayAddress.balance, amount);
        GmpMessage memory gmp = GmpMessage({
            source: 0x0,
            srcNetwork: 0,
            dest: address(0x0),
            destNetwork: 0x0,
            gasLimit: FOUNDRY_GAS_LIMIT,
            salt: 1,
            data: ""
        });
        Signature memory sig = sign(gmp);
        uint256 gasBefore = gasleft();
        (uint8 status,) = gateway.execute(sig, gmp);
        uint256 expectedRefund = (gasBefore - gasleft()) * tx.gasprice;
        uint8 GMP_STATUS_SUCCESS = 1;
        assertEq(status, GMP_STATUS_SUCCESS);
        uint256 actualRefund = mockSender.balance - amount;
        assertEq(amount - gatewayAddress.balance, actualRefund);
        assertEq(actualRefund, expectedRefund);
        assertEq(actualRefund, EXECUTE_CALL_COST);
        vm.stopPrank();
    }
}
