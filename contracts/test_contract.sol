// SPDX-License-Identifier: MIT
pragma solidity >=0.7.0 <0.9.0;

contract VotingMachine {
    uint yes_votes;
    uint no_votes;

    event YesEvent(address indexed from);
    event NoEvent(address indexed from);
    event GmpReceived(bytes4 signature);

    constructor() {
        yes_votes = 0;
        no_votes = 0;
    }

    function vote_yes() public returns (uint) {
        yes_votes += 1;
        emit YesEvent(msg.sender);
        return yes_votes;
    }

    function vote_no() public returns (uint) {
        no_votes += 1;
        emit NoEvent(msg.sender);
        return no_votes;
    }

    function get_votes_stats() public view returns (uint[] memory) {
        uint[] memory votes = new uint[](2);
        votes[0] = yes_votes;
        votes[1] = no_votes;
        return votes;
    }

    // Implementing the IGmpReceiver interface
    function onGmpReceived(bytes32 id, uint128 network, bytes32 sender, bytes calldata payload) payable external returns (bytes32) {
        require(payload.length >= 4, "Invalid payload");
        bytes4 sig = bytes4(payload);
        emit GmpReceived(sig);
        if (sig == this.vote_yes.selector) {
            vote_yes();
        } else if (sig == this.vote_no.selector) {
            vote_no();
        } else {
            revert("Invalid function signature");
        }

        // Return a result (for example, the current vote count)
        return bytes32(yes_votes << 128 | no_votes);
    }
}

// interface IGmpReceiver {
//     function onGmpReceived(bytes32 id, uint128 network, bytes32 sender, bytes calldata payload) payable external returns (bytes32);
// }
