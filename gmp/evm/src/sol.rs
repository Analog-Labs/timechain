use alloy_primitives::U256;
use time_primitives::NetworkId;

alloy_sol_types::sol! {
	#[derive(Debug, Default, PartialEq, Eq)]
	struct TssKey {
		uint8 yParity;
		uint256 xCoord;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct TssSignature {
		uint256 e;
		uint256 s;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct GmpMessage {
		uint16 srcNetwork;
		uint16 destNetwork;
		bytes32 src;
		bytes32 dest;
		uint64 nonce;
		uint128 gasLimit;
		uint128 gasCost;
		bytes data;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct Route {
		uint16 networkId;
		uint64 gasLimit;
		uint128 baseFee;
		bytes32 gateway;
		uint128 relativeGasPriceNumerator;
		uint128 relativeGasPriceDenominator;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct Network {
		uint16 id;
		address gateway;
	}

	contract GatewayProxy {
		constructor(address admin) payable;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct Signature {
		uint256 xCoord;
		uint256 e;
		uint256 s;
	}

	enum GmpStatus {
		NOT_FOUND,
		SUCCESS,
		REVERT,
		INSUFFICIENT_FUNDS,
		PENDING
	}

	contract Gateway {
		constructor(uint16 network, address proxy) payable;
		function initialize(address admin, TssKey[] memory keys, Network[] calldata networks) external;
		function deposit() external payable {}
		function upgrade(address newImplementation) external payable;
		function execute(TssSignature memory signature, uint256 xCoord, bytes memory message) external;
		function execute(Signature calldata signature, GmpMessage calldata message)
			external
			returns (GmpStatus status, bytes32 result);
		function admin() external view returns (address);
		function setAdmin(address admin) external payable;
		function shards() external view returns (TssKey[] memory);
		function setShards(TssKey[] calldata publicKeys) external;
		function routes() external view returns (Route[]);
		function setRoute(Route calldata info) external;
		function estimateMessageCost(uint16 networkid, uint256 messageSize, uint256 gasLimit) external view returns (uint256);
		function withdraw(uint256 amount, address recipient, bytes calldata data) external returns (bytes memory output);

		event ShardRegistered(TssKey key);

		event ShardUnregistered(TssKey key);

		event MessageReceived(
			bytes32 indexed id,
			GmpMessage msg
		);

		event GmpCreated(
			bytes32 indexed id,
			bytes32 indexed source,
			address indexed destinationAddress,
			uint16 destinationNetwork,
			uint256 executionGasLimit,
			uint256 salt,
			bytes data
		);

		// #[derive(Debug, Default, PartialEq, Eq)]
		// struct GmpMessage {
		// 	bytes32 foreign; destinationAddress
		// 	uint16 foreign_network; destinationNetwork
		// 	address local; source
		// 	uint128 gasLimit; execution gas limit
		// 	uint128 gasCost; we dont have
		// 	uint64 nonce; salt
		// 	bytes data; data
		// }

		event MessageExecuted(
			bytes32 indexed id,
			bytes32 indexed source,
			address indexed dest,
			uint256 status,
			bytes32 result
		);

		event BatchExecuted(
			uint64 batch,
		);
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct ProxyContext {
		uint8 v;
		bytes32 r;
		bytes32 s;
		address implementation;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct ProxyDigest {
		address proxy;
		address implementation;
	}

	contract GmpTester {
		constructor(address gateway);
		function sendMessage(GmpMessage msg) payable;
		function estimateMessageCost(uint256 messageSize, uint256 gasLimit) external view returns (uint256);
		event MessageReceived(GmpMessage msg);
	}

	// reference: https://github.com/Analog-Labs/universal-factory/blob/main/src/IUniversalFactory.sol
	interface IUniversalFactory {
		function create2(bytes32 salt, bytes calldata creationCode) external payable returns (address);
		function create2(bytes32 salt, bytes calldata creationCode, bytes calldata arguments, bytes calldata callback)
			external
			payable
			returns (address);
	}
}

pub fn u256(bytes: &[u8]) -> U256 {
	U256::from_be_bytes(<[u8; 32]>::try_from(bytes).unwrap())
}

fn bytes32(u: U256) -> [u8; 32] {
	u.to_be_bytes::<32>()
}

impl From<time_primitives::TssPublicKey> for TssKey {
	fn from(key: time_primitives::TssPublicKey) -> Self {
		Self {
			yParity: if key[0] % 2 == 0 { 0 } else { 1 },
			xCoord: u256(&key[1..]),
		}
	}
}

impl From<TssKey> for time_primitives::TssPublicKey {
	fn from(key: TssKey) -> Self {
		let mut public = [0; 33];
		public[0] = key.yParity;
		public[1..].copy_from_slice(&bytes32(key.xCoord));
		public
	}
}

impl From<time_primitives::TssSignature> for TssSignature {
	fn from(sig: time_primitives::TssSignature) -> Self {
		Self {
			e: u256(&sig[..32]),
			s: u256(&sig[32..]),
		}
	}
}

impl From<TssSignature> for time_primitives::TssSignature {
	fn from(sig: TssSignature) -> Self {
		let mut bytes = [0; 64];
		bytes[..32].copy_from_slice(&bytes32(sig.e));
		bytes[32..].copy_from_slice(&bytes32(sig.s));
		bytes
	}
}

impl From<time_primitives::Route> for Route {
	fn from(route: time_primitives::Route) -> Self {
		Self {
			networkId: route.network_id,
			gateway: route.gateway.into(),
			relativeGasPriceNumerator: route.relative_gas_price.0,
			relativeGasPriceDenominator: route.relative_gas_price.1,
			gasLimit: route.gas_limit,
			baseFee: route.base_fee,
		}
	}
}

impl From<Route> for time_primitives::Route {
	fn from(route: Route) -> Self {
		Self {
			network_id: route.networkId,
			gateway: route.gateway.into(),
			relative_gas_price: (
				route.relativeGasPriceNumerator,
				route.relativeGasPriceDenominator,
			),
			gas_limit: route.gasLimit,
			base_fee: route.baseFee,
		}
	}
}

impl From<GmpMessage> for time_primitives::GmpMessage {
	fn from(msg: GmpMessage) -> Self {
		Self {
			src_network: msg.srcNetwork,
			dest_network: msg.destNetwork,
			src: msg.src.into(),
			dest: msg.dest.into(),
			nonce: msg.nonce,
			gas_limit: msg.gasLimit,
			gas_cost: msg.gasCost,
			bytes: msg.data.into(),
		}
	}
}

impl From<time_primitives::GmpMessage> for GmpMessage {
	fn from(msg: time_primitives::GmpMessage) -> Self {
		Self {
			srcNetwork: msg.src_network,
			destNetwork: msg.dest_network,
			src: msg.src.into(),
			dest: msg.dest.into(),
			nonce: msg.nonce,
			gasLimit: msg.gas_limit,
			gasCost: msg.gas_cost,
			data: msg.bytes.into(),
		}
	}
}
