use alloy_primitives::U256;
use time_primitives::NetworkId;

alloy_sol_types::sol! {
	#[derive(Debug, Default, PartialEq, Eq)]
	struct TssPublicKey {
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
		bytes32 foreign;
		uint16 foreign_network;
		address local;
		uint128 gasLimit;
		uint128 gasCost;
		uint64 nonce;
		bytes data;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct Route {
		uint16 networkId;
		bytes32 gateway;
		uint128 relativeGasPriceNumerator;
		uint128 relativeGasPriceDenominator;
		uint64 gasLimit;
		uint128 baseFee;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct TssKey {
		uint8 yParity;
		uint256 xCoord;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct Network {
		uint16 id;
		address gateway;
	}

	contract GatewayProxy {
		constructor(address admin) payable;
	}

	contract Gateway {
		constructor(address proxy) payable;
		function initialize(address admin, TssKey[] memory keys, Network[] calldata networks) external;
		function deposit() external payable {}
		function upgrade(address newImplementation) external payable;
		function execute(TssSignature memory signature, uint256 xCoord, bytes memory message) external;
		function admin() external view returns (address);
		function setAdmin(address admin) external payable;
		function shards() external view returns (TssPublicKey[]);
		function setShards(TssPublicKey[] memory shards) external payable;
		function routes() external view returns (Route[]);
		function setRoute(Route memory route) external;
		function estimateMessageCost(uint16 networkid, uint256 messageSize, uint256 gasLimit) external view returns (uint256);
		function withdraw(uint256 amount, address recipient, bytes calldata data) external returns (bytes memory output);

		event ShardRegistered(TssPublicKey key);

		event ShardUnregistered(TssPublicKey key);

		event MessageReceived(
			bytes32 indexed id,
			GmpMessage msg
		);

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

	#[derive(Debug, Default, PartialEq, Eq)]
	struct GatewayContext {
		uint16 networkId;
	}

	contract GmpTester {
		constructor(address gateway);
		function sendMessage(GmpMessage msg) payable;
		event MessageReceived(GmpMessage msg);
	}

	// reference: https://github.com/Analog-Labs/universal-factory/blob/main/src/IUniversalFactory.sol
	interface IUniversalFactory {
		function create2(bytes32 salt, bytes calldata creationCode, bytes calldata arguments) external payable returns (address);
		function create2(bytes32 salt, bytes calldata creationCode, bytes calldata arguments, bytes calldata callback)
		external
		payable
		returns (address);
	}

}

fn u256(bytes: &[u8]) -> U256 {
	U256::from_be_bytes(<[u8; 32]>::try_from(bytes).unwrap())
}

fn bytes32(u: U256) -> [u8; 32] {
	u.to_be_bytes::<32>()
}

impl From<time_primitives::TssPublicKey> for TssPublicKey {
	fn from(key: time_primitives::TssPublicKey) -> Self {
		Self {
			yParity: if key[0] % 2 == 0 { 0 } else { 1 },
			xCoord: u256(&key[1..]),
		}
	}
}

impl From<TssPublicKey> for time_primitives::TssPublicKey {
	fn from(key: TssPublicKey) -> Self {
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

impl GmpMessage {
	pub fn outbound(self, local_network: NetworkId) -> time_primitives::GmpMessage {
		time_primitives::GmpMessage {
			dest_network: self.foreign_network,
			dest: self.foreign.into(),
			src_network: local_network,
			src: crate::t_addr(self.local),
			gas_limit: self.gasLimit,
			gas_cost: self.gasCost,
			nonce: self.nonce,
			bytes: self.data.into(),
		}
	}

	pub fn inbound(self, local_network: NetworkId) -> time_primitives::GmpMessage {
		time_primitives::GmpMessage {
			src_network: self.foreign_network,
			src: self.foreign.into(),
			dest_network: local_network,
			dest: crate::t_addr(self.local),
			gas_limit: self.gasLimit,
			gas_cost: self.gasCost,
			nonce: self.nonce,
			bytes: self.data.into(),
		}
	}

	pub fn from_inbound(msg: time_primitives::GmpMessage) -> Self {
		Self {
			foreign_network: msg.src_network,
			foreign: msg.src.into(),
			local: crate::a_addr(msg.dest),
			gasLimit: msg.gas_limit,
			gasCost: msg.gas_cost,
			nonce: msg.nonce,
			data: msg.bytes.into(),
		}
	}

	pub fn from_outbound(msg: time_primitives::GmpMessage) -> Self {
		Self {
			foreign_network: msg.dest_network,
			foreign: msg.dest.into(),
			local: crate::a_addr(msg.src),
			gasLimit: msg.gas_limit,
			gasCost: msg.gas_cost,
			nonce: msg.nonce,
			data: msg.bytes.into(),
		}
	}
}
