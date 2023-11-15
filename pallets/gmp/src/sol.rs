alloy_sol_macro::sol! {
	#[derive(Default, Debug, PartialEq, Eq)]
	struct Signature {
		uint8 parity;
		uint256 px;
		uint256 e;
		uint256 s;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct GMPMessage {
		uint128 nonce;
		uint128 networkId; // source network id
		bytes32 sender;    // sender public key
		address dest;      // dest contract
		bytes payload;     // message payload
	}

	#[derive(Debug, PartialEq, Eq)]
	interface IGateway {
		function registerTSSKeys(Signature memory signature, uint8[] memory tssKeys) external;
		function revokeTSSKeys(Signature memory signature, uint8[] memory tssKeys) external;
		function execute(Signature memory signature, GMPMessage memory message) external;
	}
}
