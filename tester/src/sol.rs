use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol, SolCall, SolConstructor};

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
		function registerGmpContract(GmpVotingContract memory _registered) external;
		function vote(bool _vote) external payable;
		function vote_wihtout_gmp(bool _vote) external;
		function stats() public view returns (uint256[] memory);
	}
}

sol! {
	contract GatewayProxy {
		constructor(address implementation, bytes memory initializer) payable;
	}
}

sol! {
	#[derive(Debug, PartialEq, Eq)]
	struct Network {
		uint16 id;
		address gateway;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct TssKey {
		uint8 yParity;
		uint256 xCoord;
	}

	type UFloat9x56 is uint64;

	#[derive(Debug, Default, PartialEq, Eq)]
	struct UpdateNetworkInfo {
		uint16 networkId;
		bytes32 domainSeparator;
		uint64 gasLimit;
		UFloat9x56 relativeGasPrice;
		uint128 baseFee;
		uint64 mortality;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct Signature {
		uint256 xCoord;
		uint256 e;
		uint256 s;
	}

	contract Gateway {
		function initialize(address admin, TssKey[] memory keys, Network[] calldata networks) external;
		function upgrade(address newImplementation) external payable;
		function setAdmin(address newAdmin) external payable;
		function sudoAddShards(TssKey[] memory shards) external payable;
		function estimateMessageCost(uint16 networkid, uint256 messageSize, uint256 gasLimit) external view returns (uint256);
		function setNetworkInfo(UpdateNetworkInfo calldata info) external;
	}
}
