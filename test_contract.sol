// SPDX-License-Identifier: MIT
pragma solidity >=0.7.0 <0.9.0;

import "contracts/gateway.sol";

struct GmpVotingContract {
    address dest;
    uint16 network;
}

contract VotingContract {
    // Minium gas required for execute the `onGmpReceived` method
    uint256 public constant GMP_GAS_LIMIT = 100_000;

    address owner;
    IGateway gateway;
    address[] dests;
    uint16[] networks;
    bool started;
    uint256 yes_votes;
    uint256 no_votes;

    event LocalVote(address indexed from, bool vote);
    event GmpVote(bytes32 id, uint128 network, bytes32 indexed sender, bool vote);
    event ResultChanged(bool vote);

    constructor(address _gateway) {
        owner = msg.sender;
        gateway = IGateway(_gateway);
        started = false;
        yes_votes = 0;
        no_votes = 0;
    }

    function registerGmpContracts(GmpVotingContract[] memory _registered) external {
        require(msg.sender == owner && started == false);
        dests = new address[](_registered.length);
        networks = new uint16[](_registered.length);
        for (uint256 i = 0; i < _registered.length; i++) {
            dests[i] = _registered[i].dest;
            networks[i] = _registered[i].network;
        }
        started = true;
    }

    function result() public view returns (bool) {
        return yes_votes > no_votes;
    }

    function stats() public view returns (uint256[] memory) {
        uint256[] memory votes = new uint256[](2);
        votes[0] = yes_votes;
        votes[1] = no_votes;
        return votes;
    }

    function _handle_vote(bool _vote) private {
        require(started);
        bool prev_result = result();
        if (_vote) {
            yes_votes += 1;
        } else {
            no_votes += 1;
        }
        bool new_result = result();
        if (prev_result != new_result) {
            emit ResultChanged(new_result);
        }
    }

    function vote(bool _vote) external {
        _handle_vote(_vote);
        bytes memory payload = abi.encode(_vote);
        for (uint256 i = 0; i < dests.length; i++) {
            gateway.submitMessage(dests[i], networks[i], GMP_GAS_LIMIT, payload);
        }
        emit LocalVote(msg.sender, _vote);
    }

    // Implementing the IGmpReceiver interface
    function onGmpReceived(bytes32 id, uint128 network, bytes32 sender, bytes calldata payload)
        external
        payable
        returns (bytes32)
    {
        require(payload.length == 32, "Invalid payload");
        (bool _vote) = abi.decode(payload, (bool));
        _handle_vote(_vote);
        emit GmpVote(id, network, sender, _vote);

        // 128bit for yes votes, 128bit for no votes
        uint256 votes = (yes_votes << 128) | uint256(uint128(no_votes));
        return bytes32(votes);
    }
}
