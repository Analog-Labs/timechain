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

fn encode_float(m: u64, e: i16) -> f64 {
	m as f64 * (e as f64).exp2()
}

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
		let gas_price = Gateway::GasPrice::from(route.gas_price);
		Self {
			networkId: route.network_id,
			gateway: route.gateway.into(),
			maxGasLimit: route.max_gas_limit,
			msgGas: route.msg_gas,
			msgByteGas: route.msg_byte_gas,
			gasPriceMantissa: gas_price.mantissa,
			gasPriceExponent: gas_price.exponent,
			msgFee: route.msg_fee,
		}
	}
}

impl From<f64> for Gateway::GasPrice {
	fn from(gas_price: f64) -> Self {
		let (mantissa, exponent, _) = num_traits::Float::integer_decode(gas_price);
		Self { mantissa, exponent }
	}
}

impl From<Gateway::GasPrice> for f64 {
	fn from(gas_price: Gateway::GasPrice) -> Self {
		encode_float(gas_price.mantissa, gas_price.exponent)
	}
}

impl From<Gateway::Route> for time_primitives::Route {
	fn from(route: Gateway::Route) -> Self {
		Self {
			network_id: route.networkId,
			gateway: route.gateway.into(),
			max_gas_limit: route.maxGasLimit,
			msg_gas: route.msgGas,
			msg_byte_gas: route.msgByteGas,
			gas_price: encode_float(route.gasPriceMantissa, route.gasPriceExponent),
			msg_fee: route.msgFee,
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_float_encoding() {
		let cases = [0., 0.1, 1., f64::MAX, f64::MIN_POSITIVE];
		for case in cases {
			let (m, e, _) = num_traits::Float::integer_decode(case);
			let f = encode_float(m, e);
			let (m2, e2, _) = num_traits::Float::integer_decode(f);
			println!("m {m} {m2} e {e:b} {e2:b}");
			assert_eq!(case, f);
		}
	}
}
