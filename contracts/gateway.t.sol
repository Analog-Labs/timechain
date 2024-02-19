pragma solidity ^0.8.20;

import "contracts/gateway.sol";
import "forge-std/Test.sol";

uint256 constant parity = 0x0;
uint256 constant xCoord = 0x3a4e117530d6dd0f7ab5b8dfd2f13c91ccdca76dcd95d235651bdfff490c1e26;

contract GatewayTest is Test {
    Gateway gateway;
    
    constructor() {
        uint256[2][] memory keys = new uint256[2][](1);
        keys[0] = [parity, xCoord];
        gateway = new Gateway(keys);
    }

    function test_something() public {
        assertEq(parity, 0x0);
    }
}
