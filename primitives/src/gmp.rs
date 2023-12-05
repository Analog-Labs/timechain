use crate::{ChainId, Function, TssSignature};
pub use alloy_primitives::U256;
use alloy_sol_types::SolCall;
use serde::{Deserialize, Serialize};
use sp_core::keccak_256;
use sp_std::vec::Vec;
use IGateway::*;

/// Make Function::SendMessage that aligns with Gateway contract interface
pub fn make_gmp(
	shard_nonce: u32,
	raw_payload: WrappedGmpPayload,
) -> Function {
	let domain_separator = keccak_256(
		[
			keccak_256(b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
			keccak_256(b"Analog Gateway Contract"),
			keccak_256(b"0.1.0"),
			raw_payload.dest_network.into(),
			raw_payload.dest.as_slice(),
		].concat()
	);
	let gmp: GmpPayload = raw_payload.into();
	let payload_hash = keccak_256(
		[
			keccak_256(
				b"GMPPayload(bytes32 source,uint96 srcNetwork,address dest,uint96 destNetwork,bytes32 sender,uint256 gasLimit,uint256 value,uint256 salt,bytes data)"
			),
			gmp.source.as_bytes(),
			gmp.srcNetwork.as_bytes(),
			gmp.dest.as_bytes(),
			gmp.destNetwork.as_bytes(),
			gmp.gasLimit.as_bytes(),
			gmp.salt.as_bytes(),
			keccak_256(&gmp.data),
		].concat()
	);

	let gmp_eip712_message: Vec<u8> = [
		[0x19, 0x01],
		domain_separator,
		keccak_256(
			[
				keccak_256(b"GmpMessage(uint32 nonce,GmpPayload payload)"),
				shard_nonce.as_bytes(),
				payload_hash,
			]
			.concat(),
		),
	];
	Function::SendMessage {
		contract_address: raw_payload.dest.to_vec(),
		payload: gmp_eip712_message,
	}
}

pub fn sudo_register_shard(
	shard_public_key: [u8; 33],
	shard_nonce: u32,
	source: [u8; 32],
	src_network: U256,
	dest: [u8; 20],
	dest_network: U256,
	sender: [u8; 32],
	gas_limit: U256,
	salt: U256,
) -> Function {
	make_gmp(
		shard_nonce,
		WrappedGmpPayload {
			source,
			src_network,
			dest,
			dest_network,
			sender,
			gas_limit,
			salt,
			data: sudoUpdateTSSKeysCall {
				message: UpdateShardsMessage {
					nonce: shard_nonce,
					register: sp_std::vec![shard_public_key.into()],
					revoke: sp_std::vec![],
				}
			}
			.abi_encode(),
		}.into()
	)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedGmpPayload {
	pub source: [u8; 32],
	pub src_network: ChainId,
	pub dest: [u8; 20],
	pub dest_network: ChainId,
	pub sender: [u8; 32],
	pub gas_limit: U256,
	pub salt: U256,
	pub data: Vec<u8>,
}

impl From<WrappedGmpPayload> for GmpPayload {
	fn from(wrapped_payload: WrappedGmpPayload) -> GmpPayload {
		GmpPayload {
			source: wrapped_payload.source.into(),
			srcNetwork: wrapped_payload.src_network.into(),
			dest: wrapped_payload.dest.into(),
			destNetwork: wrapped_payload.dest_network.into(),
			sender: wrapped_payload.sender.into(),
			gasLimit: wrapped_payload.gas_limit,
			salt: wrapped_payload.salt,
			data: wrapped_payload.data,
		}
	}
}

// TODO: uncomment and debug
// impl From<crate::TssSignature> for Signature {
// 	fn from(signature: TssSignature) -> Signature {
// 		Signature {
// 			parity: signature.parity,
// 			px: signature.px,
// 			e: signature.e,
// 			s: signature.s,
// 		}
// 	}
// }

impl From<[u8; 33]> for TssKey {
	fn from(key: [u8; 33]) -> TssKey {
		let mut key_vec = key.to_vec();
		let x_coordinate: [u8; 32] = key[1..].try_into().unwrap();
		let (parity, coord_x) = (key_vec.remove(0), x_coordinate.into());
		TssKey { parity, coordX: coord_x }
	}
}

alloy_sol_macro::sol! {
	#[derive(Default, Debug, PartialEq, Eq)]
	struct Signature {
		uint8 parity;
		uint256 px;
		uint256 e;
		uint256 s;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct TssKey {
		uint8 parity;    // public key y-coord parity (27 or 28)
		bytes32 coordX;  // public key x-coord
	}

	/**
	* @dev Message used to revoke or/and register new shards
	*/
	#[derive(Debug, PartialEq, Eq)]
	struct UpdateShardsMessage {
		uint32 nonce;       // shard's nonce to prevent replay attacks
		TssKey[] revoke;    // Keys to revoke
		TssKey[] register;  // Keys to add
	}

	/**
	* @dev GMP payload, this is what the timechain creates as task payload
	*/
	#[derive(Debug, PartialEq, Eq)]
	struct GmpPayload {
		bytes32 source;      // Pubkey/Address of who send the GMP message
		uint128 srcNetwork;  // Source chain identifier (it's the EIP-155 chain_id for ethereum networks)
		address dest;        // Destination/Recipient contract address
		uint128 destNetwork; // Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
		bytes32 sender;      // Sender address or public key
		uint256 gasLimit;    // gas limit of the GMP call
		uint256 salt;        // Message salt, useful for sending two messages with same content
		bytes data;          // message data with no specified format
	}

	/**
	* @dev GMP message, this is what the shard signs
	*/
	#[derive(Debug, PartialEq, Eq)]
	struct GmpMessage {
		uint32 nonce;
		GmpPayload payload;
	}

	#[derive(Debug, PartialEq, Eq)]
	interface IGateway {
		function updateTSSKeys(Signature memory signature, UpdateShardsMessage memory message) external;
		function execute(Signature memory signature, GmpPayload memory message) external returns (bool success);
		function sudoUpdateTSSKeys(UpdateShardsMessage memory message) external;
		function sudoExecute(GmpMessage memory message) external returns (bool success);
	}
}
