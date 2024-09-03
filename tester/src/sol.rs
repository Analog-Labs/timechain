use alloy_sol_types::sol;

sol! {
	#[derive(Debug, PartialEq, Eq)]
	struct GmpVotingContract {
		address dest;
		uint16 network;
	}

	contract VotingContract {
		// Minium gas required for execute the `onGmpReceived` method
		function GMP_GAS_LIMIT() external pure returns(uint256);

		constructor(address _gateway);
		function deposit() external payable;
		function registerGmpContract(GmpVotingContract memory _registered) external;
		function vote(bool _vote) external payable;
		function vote_wihtout_gmp(bool _vote) external;
		function stats() public view returns (uint256[] memory);
	}
}
