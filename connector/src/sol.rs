use alloy_sol_types::sol;

sol! {
	#[derive(Debug, Default, PartialEq, Eq)]
	struct TssKey {
		uint8 yParity;
		uint256 xCoord;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct UpdateKeysMessage {
		TssKey[] revoke;
		TssKey[] register;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct GmpMessage {
		bytes32 source;
		uint16 srcNetwork;
		address dest;
		uint16 destNetwork;
		uint256 gasLimit;
		uint256 salt;
		bytes data;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct Signature {
		uint256 xCoord;
		uint256 e;
		uint256 s;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct Network {
		uint16 id;
		address gateway;
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

	interface IGateway {
		event GmpExecuted(
			bytes32 indexed id,
			bytes32 indexed source,
			address indexed dest,
			uint256 status,
			bytes32 result
		);

		event KeySetChanged(
			bytes32 indexed id,
			TssKey[] revoked,
			TssKey[] registered
		);

		event GmpCreated(
			bytes32 indexed id,
			bytes32 indexed sender,
			address indexed recipient,
			uint16 network,
			uint256 gasLimit,
			uint256 salt,
			bytes data
		);

		constructor(uint16 networkId, address proxy) payable;
		function execute(Signature memory signature, GmpMessage memory message) external returns (uint8 status, bytes32 result);
		function updateKeys(Signature memory signature, UpdateKeysMessage memory message) external;
		function deposit() public payable;
	}

	contract Gateway {
		function initialize(address admin, TssKey[] memory keys, Network[] calldata networks) external;
		function upgrade(address newImplementation) external payable;
		function setAdmin(address newAdmin) external payable;
		function sudoAddShards(TssKey[] memory shards) external payable;
		function estimateMessageCost(uint16 networkid, uint256 messageSize, uint256 gasLimit) external view returns (uint256);
		function setNetworkInfo(UpdateNetworkInfo calldata info) external;
	}

	contract GatewayProxy {
		constructor(address implementation, bytes memory initializer) payable;
	}

	contract GmpTester {
		constructor(address gateway);
		function sendMessage(GmpMessage msg) payable;
		event RecvMessage(GmpMessage msg);
	}
}

pub fn addr(addr: time_primitives::Address) -> alloy_primitives::Address {
	alloy_primitives::Address(addr[12..].try_into().unwrap())
}

pub fn event_to_gmp_message(
	event: IGateway::GmpCreated,
	src_network: u16,
) -> time_primitives::GmpMessage {
	let mut dest = [0; 32];
	dest[12..].copy_from_slice(&event.recipient.0 .0[..]);
	time_primitives::GmpMessage {
		src_network,
		src: event.sender.0,
		dest_network: event.network,
		dest,
		nonce: event.salt.to(),
		gas_limit: u128::try_from(event.gasLimit).unwrap_or(u128::MAX),
		bytes: event.data.into(),
	}
}
