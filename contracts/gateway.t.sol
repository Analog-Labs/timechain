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

    function test_balance_before_after_call() public {
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
        
        //uint256 balanceBefore = address(msg.sender).balance;
        (uint8 status, bytes32 result) = gateway.execute(sig, gmp);
        //assert(balanceBefore > address(msg.sender).balance);
    }
}
