// SPDX-License-Identifier: MIT
pragma solidity ^0.8.7;


contract TokenSwap {

    function getAmountsOut(address from,address to, int32 _amount) external pure returns(int32[] memory) {
        require(from!=address(0),"Zero Address");
        require(to!=address(0),"Zero Address");
        int32[] memory price  = new int32[](2);
        price[0] = _amount*2;
        price[1] = _amount*3;
        return price;
    }
    
    function decimals() external pure returns(int32 ) {
        return 18;
    }

    
    
    function getValue() external pure returns(string memory) {
        return "hello world";
    }
}
