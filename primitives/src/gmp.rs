use crate::{Function, Msg, NetworkId, TssPublicKey, TssSignature};
use alloy_primitives::private::Vec;
use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol, Eip712Domain, SolCall, SolStruct};

const EIP712_NAME: &str = "Analog Gateway Contract";
const EIP712_VERSION: &str = "0.1.0";

pub type Eip712Bytes = [u8; 66];
pub type Eip712Hash = [u8; 32];

pub struct GmpParams {
	pub network_id: NetworkId,
	pub gateway_contract: Address,
	pub tss_public_key: TssPublicKey,
}

impl GmpParams {
	fn eip712_domain_separator(&self) -> Eip712Domain {
		Eip712Domain {
			name: Some(EIP712_NAME.into()),
			version: Some(EIP712_VERSION.into()),
			chain_id: Some(U256::from(self.network_id)),
			verifying_contract: Some(self.gateway_contract),
			salt: None,
		}
	}

	fn to_eip712_bytes(&self, hash: Eip712Hash) -> Eip712Bytes {
		let mut digest_input = [0u8; 2 + 32 + 32];
		digest_input[0] = 0x19;
		digest_input[1] = 0x01;
		digest_input[2..34].copy_from_slice(&self.eip712_domain_separator().hash_struct()[..]);
		digest_input[34..66].copy_from_slice(&hash[..]);
		digest_input
	}
}

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

		function execute(Signature memory signature, GmpMessage memory message) external returns (uint8 status, bytes32 result);
		function updateKeys(Signature memory signature, UpdateKeysMessage memory message) external;
		function deposit(bytes32 source, uint16 network) public payable;
	}
}

impl From<TssPublicKey> for TssKey {
	fn from(bytes: TssPublicKey) -> Self {
		Self {
			yParity: if bytes[0] % 2 == 0 { 0 } else { 1 },
			xCoord: U256::from_be_slice(&bytes[1..]),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
	UpdateKeys(UpdateKeysMessage),
	Gmp(GmpMessage),
}

impl Message {
	pub fn update_keys(
		revoke: impl IntoIterator<Item = TssPublicKey>,
		register: impl IntoIterator<Item = TssPublicKey>,
	) -> Message {
		Self::UpdateKeys(UpdateKeysMessage {
			revoke: revoke.into_iter().map(TssKey::from).collect(),
			register: register.into_iter().map(TssKey::from).collect(),
		})
	}

	pub fn gmp(msg: Msg) -> Message {
		Self::Gmp(GmpMessage {
			source: msg.source.into(),
			srcNetwork: msg.source_network,
			dest: Address(msg.dest.into()),
			destNetwork: msg.dest_network,
			gasLimit: U256::from(msg.gas_limit),
			salt: U256::from_be_bytes(msg.salt),
			data: msg.data,
		})
	}

	fn payload(self, signature: Signature) -> Vec<u8> {
		match self {
			Self::UpdateKeys(message) => {
				IGateway::updateKeysCall { message, signature }.abi_encode()
			},
			Self::Gmp(message) => IGateway::executeCall { message, signature }.abi_encode(),
		}
	}

	fn eip712_hash_struct(&self) -> [u8; 32] {
		match self {
			Self::UpdateKeys(msg) => msg.eip712_hash_struct().into(),
			Self::Gmp(msg) => msg.eip712_hash_struct().into(),
		}
	}

	pub fn to_eip712_bytes(&self, params: &GmpParams) -> Eip712Bytes {
		let hash = self.eip712_hash_struct();
		params.to_eip712_bytes(hash)
	}

	/// Converts `Message` into `Function::EvmCall`
	pub fn into_evm_call(self, params: &GmpParams, signature: TssSignature) -> Function {
		let signature = Signature {
			xCoord: TssKey::from(params.tss_public_key).xCoord,
			e: U256::from_be_slice(&signature[0..32]),
			s: U256::from_be_slice(&signature[32..64]),
		};
		let gas_limit = if let Self::Gmp(GmpMessage { gasLimit, .. }) = &self {
			let gas_limit = u64::try_from(*gasLimit).unwrap_or(u64::MAX).saturating_add(100_000);
			Some(gas_limit.min(29_900_000))
		} else {
			None
		};
		let payload = self.payload(signature);
		Function::EvmCall {
			address: params.gateway_contract.into(),
			input: payload,
			amount: 0u128,
			gas_limit,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_payload() {
		let msg = Msg {
			source_network: 42,
			source: [0; 32],
			dest_network: 69,
			dest: [0; 20],
			gas_limit: 0,
			salt: [0; 32],
			data: vec![],
		};
		let params = GmpParams {
			network_id: msg.dest_network,
			gateway_contract: [0; 20].into(),
			tss_public_key: [0; 33],
		};
		let expected_bytes = "19013e3afdf794f679fcbf97eba49dbe6b67cec6c7d029f1ad9a5e1a8ffefa8db2724ed044f24764343e77b5677d43585d5d6f1b7618eeddf59280858c68350af1cd";
		let bytes = Message::gmp(msg).to_eip712_bytes(&params);
		assert_eq!(hex::encode(bytes), expected_bytes);
	}
}
