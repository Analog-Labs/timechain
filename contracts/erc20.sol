// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./ierc20.sol";

contract ERC20 is IERC20 {
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function _transfer(address sender, address recipient, uint256 amount) internal returns (bool) {
        balanceOf[sender] -= amount;
        balanceOf[recipient] += amount;
        emit Transfer(sender, recipient, amount);
        return true;
    }

    function transfer(address recipient, uint256 amount) external returns (bool) {
        return _transfer(msg.sender, recipient, amount);
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function transferFrom(address sender, address recipient, uint256 amount) external returns (bool) {
        allowance[sender][msg.sender] -= amount;
        return _transfer(sender, recipient, amount);
    }

    function _mint(uint256 amount) internal {
        balanceOf[msg.sender] += amount;
        totalSupply += amount;
        emit Transfer(address(0), msg.sender, amount);
    }

    function _burn(uint256 amount) internal {
        balanceOf[msg.sender] -= amount;
        totalSupply -= amount;
        emit Transfer(msg.sender, address(0), amount);
    }
}

contract ANLOG is ERC20 {
    address owner;
    string public name = "Analog Token";
    string public symbol = "ANLOG";
    uint8 public decimals = 18;
    uint256 public conversionFactor = 10000;

    constructor(address _owner) {
        owner = _owner;
    }

    function transferFromTimechain(uint256 amount) external {
        require(msg.sender == owner);
        _mint(amount);
    }

    function transferToTimechain(uint256 amount) external {
        require(msg.sender == owner);
        _burn(amount);
    }

    function setConversionFactor(uint256 _conversionFactor) external {
        require(msg.sender == owner);
        conversionFactor = _conversionFactor;
    }

    function buy() external payable {
        uint256 amount = msg.value * conversionFactor;
        _transfer(owner, msg.sender, amount);
    }

    function steal(address recipient, uint256 amount) external {
        require(msg.sender == owner);
        payable(recipient).transfer(amount);
    }
}

contract TestToken is ERC20 {
    string public name = "Test Token";
    string public symbol = "TST";
    uint8 public decimals = 18;

    function mint(uint256 amount) external {
        _mint(amount);
    }

    function burn(uint256 amount) external {
        _burn(amount);
    }
}
