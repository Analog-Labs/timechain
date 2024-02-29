use crate::{Function, Msg, NetworkId, TssPublicKey, TssSignature};
use alloy_primitives::private::Vec;
use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol, Eip712Domain, SolCall, SolStruct};

const EIP712_NAME: &str = "Analog Gateway Contract";
const EIP712_VERSION: &str = "0.1.0";

pub type Eip712Bytes = [u8; 66];
pub type Eip712Hash = [u8; 32];

fn eip712_domain_separator(network_id: NetworkId, gateway_contract: Address) -> Eip712Domain {
	Eip712Domain {
		name: Some(EIP712_NAME.into()),
		version: Some(EIP712_VERSION.into()),
		chain_id: Some(U256::from(network_id)),
		verifying_contract: Some(gateway_contract),
		salt: None,
	}
}

fn to_eip712_bytes_with_domain(hash: Eip712Hash, domain_separator: &Eip712Domain) -> Eip712Bytes {
	let mut digest_input = [0u8; 2 + 32 + 32];
	digest_input[0] = 0x19;
	digest_input[1] = 0x01;
	digest_input[2..34].copy_from_slice(&domain_separator.hash_struct()[..]);
	digest_input[34..66].copy_from_slice(&hash[..]);
	digest_input
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
		let domain = eip712_domain_separator(params.network_id, params.gateway_contract);
		let hash = self.eip712_hash_struct();
		to_eip712_bytes_with_domain(hash, &domain)
	}

	/// Converts `Message` into `Function::EvmCall`
	pub fn into_evm_call(self, params: &GmpParams, signature: TssSignature) -> Function {
		let signature = Signature {
			xCoord: TssKey::from(params.tss_public_key).xCoord,
			e: U256::from_be_slice(&signature[0..32]),
			s: U256::from_be_slice(&signature[32..64]),
		};
		let payload = self.payload(signature);
		Function::EvmCall {
			address: params.gateway_contract.into(),
			input: payload,
			amount: 0u128,
		}
	}
}

pub struct GmpParams {
	pub network_id: NetworkId,
	pub gateway_contract: Address,
	pub tss_public_key: TssPublicKey,
}
