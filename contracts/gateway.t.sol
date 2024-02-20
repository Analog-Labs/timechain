pragma solidity ^0.8.20;

import "contracts/gateway.sol";
import "frost-evm/sol/Signer.sol";
import "forge-std/Test.sol";

uint256 constant secret = 0x42;
uint256 constant nonce = 0x69;

contract GatewayTest is Test {
    Gateway gateway;
    Signer signer;
    
    constructor() {
        signer = new Signer(secret);
        uint256[2][] memory keys = new uint256[2][](1);
        keys[0] = [signer.yParity() == 28 ? 1 : 0, signer.xCoord()];
        gateway = new Gateway(keys);
    }

    function sign(GmpMessage memory gmp) internal view returns (Signature memory) {
        uint256 hash = uint256(gateway.getGmpTypedHash(gmp));
        (uint256 e, uint256 s) = signer.signPrehashed(hash, nonce);
        return Signature({
            xCoord: signer.xCoord(),
            e: e,
            s: s
        });
    }

    function testExecuteWithoutDeposit() public {
        GmpMessage memory gmp = GmpMessage({
                source: 0x0,
                srcNetwork: 0,
                dest: address(msg.sender),
                destNetwork: uint128(block.chainid),
                gasLimit: 100000,
                salt: 1,
                data: ""
        });
        Signature memory sig = sign(gmp);
        vm.expectRevert(bytes("deposit below max refund"));
        gateway.execute(sig, gmp);
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
        assertEq(
            gatewayAddress.balance - gatewayBalanceBefore, amount,
            "deposit failed to transfer amount to gateway"
        );
        vm.stopPrank();
    }

    function testExecuteRefundsInFull() public {
        uint256 amount = 100 ether;
        address mockSender = address(0x0);
        vm.deal(mockSender, amount);
        vm.startPrank(mockSender);
        gateway.deposit{value: amount}(0x0, 0);
        GmpMessage memory gmp = GmpMessage({
                source: 0x0,
                srcNetwork: 0,
                dest: address(mockSender),
                destNetwork: uint128(block.chainid),
                gasLimit: 100000,
                salt: 1,
                data: ""
        });
        Signature memory sig = sign(gmp);
        uint256 gasBefore = gasleft();
        uint256 senderBalanceBefore = address(mockSender).balance;
        (uint8 status,) = gateway.execute(sig, gmp);
        assert(gasBefore > gasleft());
        // fully refunded mockSender
        assertEq(senderBalanceBefore, address(mockSender).balance);
        uint8 GMP_STATUS_SUCCESS = 1;
        assertEq(status, GMP_STATUS_SUCCESS);
        vm.stopPrank();
    }
}
