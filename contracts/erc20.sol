// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./ierc20.sol";

contract ERC20 is IERC20 {
    uint public totalSupply;
    mapping(address => uint) public balanceOf;
    mapping(address => mapping(address => uint)) public allowance;

    function transfer(address recipient, uint amount) external returns (bool) {
        balanceOf[msg.sender] -= amount;
        balanceOf[recipient] += amount;
        emit Transfer(msg.sender, recipient, amount);
        return true;
    }

    function approve(address spender, uint amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function transferFrom(
        address sender,
        address recipient,
        uint amount
    ) external returns (bool) {
        allowance[sender][msg.sender] -= amount;
        balanceOf[sender] -= amount;
        balanceOf[recipient] += amount;
        emit Transfer(sender, recipient, amount);
        return true;
    }

    function _mint(uint amount) internal {
        balanceOf[msg.sender] += amount;
        totalSupply += amount;
        emit Transfer(address(0), msg.sender, amount);
    }

    function _burn(uint amount) internal {
        balanceOf[msg.sender] -= amount;
        totalSupply -= amount;
        emit Transfer(msg.sender, address(0), amount);
    }
}

contract TestToken is ERC20 {
    string public name = "Test Token";
    string public symbol = "TST";
    uint8 public decimals = 18;

    function mint(uint amount) external {
        _mint(amount);
    }

    function burn(uint amount) external {
        _burn(amount);
    }
}

contract ANLOG is ERC20 {
    address owner;
    string public name = "Analog Token";
    string public symbol = "ANLOG";
    uint8 public decimals = 18;

    constructor(address _owner) {
        owner = _owner;
    }

    function mint(uint amount) external {
        require(msg.sender == owner);
        _mint(amount);
    }

    function burn(uint amount) external {
        require(msg.sender == owner);
        _burn(amount);
    }
}

contract ETH is ERC20 {
    string public name = "Etherum Token";
    string public symbol = "ETH";
    uint8 public decimals = 18;

    function mint() external payable {
        _mint(msg.value);
    }

    function burn(uint amount) external {
        _burn(amount);
        payable(msg.sender).transfer(amount);
    }
}
