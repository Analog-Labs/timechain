// SPDX-License-Identifier: MIT
pragma solidity >=0.4.22;

contract VotingMachine{
    uint yes_votes;
    uint no_votes;

    event YesEvent(address indexed from);
    event NoEvent(address indexed from);

    constructor() {
        yes_votes = 0;
        no_votes = 0;
    }

    function vote_yes() public {
        yes_votes += 1;
        emit YesEvent(msg.sender);
    }

    function vote_no() public {
        no_votes += 1;
        emit NoEvent(msg.sender);
    }

    function get_votes_stats() external view returns (uint[] memory) {
        uint[] memory votes = new uint[](2);
        votes[0] = yes_votes;
        votes[1] = no_votes;
        return votes;
    }
}
