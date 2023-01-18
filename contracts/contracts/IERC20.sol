// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.7;
interface IERC20 {
    //Returns the amount of tokens in existence.
    function totalSupply() external view returns (uint256);
    //Returns the amount of tokens owned by account.
    function balanceOf(address account) external view returns (uint256);
    //transfer token from owner account to recipient account.
    function transfer(address recipient, uint256 amount) external  returns (bool);
    function allowance(address owner, address spender)external view returns(uint256);
    function approve(address spender, uint256 amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount) external returns(bool);
    event Transfer(address from, address to, uint256 value);
    event Approval(address owner, address spender, uint256 value);
    function name() external view returns(string memory);
    function symbol() external view returns(string memory);
    function decimal() external view returns(uint256);
}

