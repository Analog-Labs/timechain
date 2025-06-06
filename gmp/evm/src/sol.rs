use crate::{a_addr, t_addr};
use alloy::{primitives::U256, sol, sol_types::SolValue};

sol!(
	#[allow(clippy::too_many_arguments)]
	#[allow(missing_docs)]
	#[derive(Debug)]
	ERC1967Proxy,
	"gateway/out/ERC1967Proxy.sol/ERC1967Proxy.json"
);

sol!(
	#[allow(clippy::too_many_arguments)]
	#[allow(missing_docs)]
	#[derive(Debug)]
	Gateway,
	"gateway/out/Gateway.sol/Gateway.json"
);

sol!(
	#[allow(clippy::too_many_arguments)]
	#[allow(missing_docs)]
	#[derive(Debug)]
	IGmpReceiver,
	"gateway/out/IGmpReceiver.sol/IGmpReceiver.json"
);

sol!(
	#[allow(clippy::too_many_arguments)]
	#[allow(missing_docs)]
	#[derive(Debug)]
	GmpProxy,
	"gateway/out/GmpProxy.sol/GmpProxy.json"
);

pub fn u256(bytes: &[u8]) -> U256 {
	U256::from_be_bytes(<[u8; 32]>::try_from(bytes).unwrap())
}

fn bytes32(u: U256) -> [u8; 32] {
	u.to_be_bytes::<32>()
}

impl From<(time_primitives::TssPublicKey, u16)> for Gateway::TssKey {
	fn from((key, num_sessions): (time_primitives::TssPublicKey, u16)) -> Self {
		Self {
			yParity: key[0] + 25,
			xCoord: u256(&key[1..]),
			numSessions: num_sessions,
		}
	}
}

impl From<Gateway::TssKey> for time_primitives::TssPublicKey {
	fn from(key: Gateway::TssKey) -> Self {
		let mut public = [0; 33];
		public[0] = key.yParity - 25;
		public[1..].copy_from_slice(&bytes32(key.xCoord));
		public
	}
}

impl From<time_primitives::Route> for Gateway::Route {
	fn from(route: time_primitives::Route) -> Self {
		Self {
			networkId: route.network_id,
			gateway: route.gateway.into(),
			relativeGasPriceNumerator: u256(&route.relative_gas_price.0.to_big_endian()),
			relativeGasPriceDenominator: u256(&route.relative_gas_price.1.to_big_endian()),
			gasLimit: route.gas_limit,
			baseFee: route.gmp_base_fee,
			gasCoef0: route.base_gas,
			gasCoef1: route.msg_byte_gas,
		}
	}
}

impl From<Gateway::Route> for time_primitives::Route {
	fn from(route: Gateway::Route) -> Self {
		Self {
			network_id: route.networkId,
			gateway: route.gateway.into(),
			relative_gas_price: (
				time_primitives::U256::from_big_endian(&bytes32(route.relativeGasPriceNumerator)),
				time_primitives::U256::from_big_endian(&bytes32(route.relativeGasPriceDenominator)),
			),
			gas_limit: route.gasLimit,
			gmp_base_fee: route.baseFee,
			base_gas: route.gasCoef0,
			msg_byte_gas: route.gasCoef1,
		}
	}
}

impl From<GmpProxy::GmpMessage> for time_primitives::GmpMessage {
	fn from(msg: GmpProxy::GmpMessage) -> Self {
		Self {
			src_network: msg.srcNetwork,
			dest_network: msg.destNetwork,
			src: msg.source.into(),
			dest: t_addr(msg.dest),
			nonce: msg.nonce,
			gas_limit: msg.gasLimit,
			bytes: msg.data.into(),
		}
	}
}

impl From<time_primitives::GmpMessage> for Gateway::GmpMessage {
	fn from(msg: time_primitives::GmpMessage) -> Self {
		Self {
			srcNetwork: msg.src_network,
			destNetwork: msg.dest_network,
			source: msg.src.into(),
			dest: a_addr(msg.dest),
			nonce: msg.nonce,
			gasLimit: msg.gas_limit,
			data: msg.bytes.into(),
		}
	}
}

impl From<time_primitives::GatewayOp> for Gateway::GatewayOp {
	fn from(msg: time_primitives::GatewayOp) -> Self {
		match msg {
			time_primitives::GatewayOp::SendMessage(msg) => Gateway::GatewayOp {
				command: 1,
				params: Into::<Gateway::GmpMessage>::into(msg).abi_encode().into(),
			},
			time_primitives::GatewayOp::RegisterShard(key, sessions) => Gateway::GatewayOp {
				command: 2,
				params: Into::<Gateway::TssKey>::into((key, sessions)).abi_encode().into(),
			},
			time_primitives::GatewayOp::UnregisterShard(key, sessions) => Gateway::GatewayOp {
				command: 3,
				params: Into::<Gateway::TssKey>::into((key, sessions)).abi_encode().into(),
			},
		}
	}
}
