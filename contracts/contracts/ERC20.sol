// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.7;
import './IERC20.sol';
contract ERC20 is IERC20 {
    mapping (address => uint) private _balances;
    mapping (address => mapping (address => uint)) private _allowance;
    uint256 private _totalSupply;
    string private _name;
    string private _symbol;
    constructor(string memory name_,string memory symbol_) {
        _name = name_;
        _symbol = symbol_;
        _mint(msg.sender, 10000*10**18);
    }
    function totalSupply() external override view returns (uint256){
        return _totalSupply;
    }
    function balanceOf(address account) external override view returns (uint256){
        return _balances[account];
    }
    function decimals() external pure returns(int32 ) {
        return 18;
    }
    function _transfer(address from, address to, uint256 amount) internal {
        require(from != address(0),"from zero address");
        require(to != address(0),"from zero address");
        uint256 fromBalance = _balances[from];
        require(fromBalance >= amount,"Insufficent amount");
        // unchecked {
            _balances[from]= fromBalance - amount;
        // }
        _balances[to]+= amount;
        emit Transfer(from,to,amount);
    }
    function transfer(address recipient, uint256 amount) external override returns (bool){
        address owner = msg.sender;
        _transfer(owner,recipient,amount);
        return true;
    }
    function allowance(address owner, address spender)external override view returns(uint256){
        return _allowance[owner][spender];
    }
    function _approve(address from,address to, uint256 amount) internal {
        require(from != address(0),"from zero address");
        require(to != address(0),"from zero address");
        _allowance[from][to] = amount;
        emit Approval(from, to, amount);
    }
    function approve(address spender, uint256 amount) external override returns (bool){
        address owner = msg.sender;
        _approve(owner,spender,amount);
        return true;
    }
    function _spendAllowance(address owner, address spender, uint256 amount) internal {
        uint256 currAllowance = this.allowance(owner, spender);
        if( currAllowance != type(uint256).max) {
            require(currAllowance >= amount,"Insufficent amount");
                _approve(owner, spender, currAllowance - amount);
        }
    }
    function transferFrom(address sender, address recipient, uint256 amount) external override returns(bool){
        address spender = msg.sender;
        _spendAllowance(sender, spender,amount);
        _transfer(sender, recipient, amount);
        return true;
    }
    function name() external override view returns(string memory){
        return _name;
    }
    function symbol() external override view returns(string memory){
        return _symbol;
    }
    function decimal() external override pure returns(uint256){
        return 18;
    }
    function _mint(address account, uint256 amount) internal {
        require(account != address(0),"zero address");
        _totalSupply+=amount;
        _balances[account]+=amount;
        emit Transfer(address(0), account, amount);
    }
}