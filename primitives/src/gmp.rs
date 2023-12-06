use crate::{ChainId, Function, TssSignature};
pub use alloy_primitives::{address, Address, U256};
use alloy_sol_types::SolCall;
use codec::alloc::string::String;
use scale_info::prelude::vec::Vec;
use serde::{Deserialize, Serialize};
use sp_io::hashing::keccak_256;
use IGateway::*;

pub fn register_shard_call(shard_public_key: [u8; 33], contract_address: [u8; 20]) -> Function {
	Function::EvmCall {
		address: String::from_utf8(contract_address.to_vec()).unwrap(),
		function_signature: String::from("execute(Signature,GmpMessage)"),
		input: sp_std::vec![String::from_utf8(
			registerTSSKeysCall {
				signature: Default::default(),
				tssKeys: sp_std::vec![shard_public_key.into()],
			}
			.abi_encode()
		)
		.unwrap()],
		// TODO estimate gas required for gateway
		amount: 1u128, // >0 so failed execution is not due to lack of gas
	}
}

/// Inserts signature into call iff it is a register shard call
/// else returns the passed input unaltered
pub fn insert_sig_iff_register_shard(input: Vec<String>, sig: TssSignature) -> Vec<String> {
	let mut signed = Vec::new();
	for arg in input {
		let try_decode_register_shard =
			registerTSSKeysCall::abi_decode(&arg.clone().into_bytes(), true);
		if let Ok(call) = try_decode_register_shard {
			// replace dummy signature but keep tssKeys unaltered
			signed.push(
				String::from_utf8(
					registerTSSKeysCall {
						signature: sig.into(),
						tssKeys: call.tssKeys,
					}
					.abi_encode(),
				)
				.unwrap(),
			);
		} else {
			signed.push(arg);
		}
	}
	signed
}

impl From<crate::TssSignature> for Signature {
	fn from(_signature: TssSignature) -> Signature {
		todo!()
	}
}

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
		uint256 xCoord;
		uint256 e;
		uint256 s;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct TssKey {
		uint8 parity;    // public key y-coord parity (27 or 28)
		bytes32 coordX;  // public key x-coord
	}

	#[derive(Debug, PartialEq, Eq)]
	struct RegisterTssKeys {
		uint256 nonce;
		TssKey[] keys;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct RevokeTssKeys {
		uint256 nonce;
		TssKey[] keys;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct GmpPayload {
		bytes32 source;      // Pubkey/Address of who send the GMP message
		uint128 srcNetwork;  // Source chain identifier (it's the EIP-155 chain_id for ethereum networks)
		address dest;        // Destination/Recipient contract address
		uint128 destNetwork; // Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
		uint256 gasLimit;    // gas limit of the GMP call
		uint256 salt;        // Message salt, useful for sending two messages with same content
		bytes data;          // message data with no specified format
	}

	#[derive(Debug, PartialEq, Eq)]
	struct GmpMessage{
		uint32 nonce;
		GmpPayload payload;
	}

	#[derive(Debug, PartialEq, Eq)]
	struct ShardInfo {
		uint128 flags;
		uint128 nonce;
	}

	#[derive(Debug, PartialEq, Eq)]
	interface IGateway {
		function sudoRegisterTSSKeys(TssKey[] memory tssKeys) external;
		function registerTSSKeys(Signature memory signature, TssKey[] memory tssKeys) external;
		function sudoRevokeTSSKeys(TssKey[] memory tssKeys);
		function revokeTSSKeys(Signature memory signature, TssKey[] memory tssKeys) external;
		function sudoExecute(GmpPayload memory message) external returns (bool success);
		function execute(Signature memory signature, GmpMessage memory message) external returns (bool success);
	}
}

impl GmpMessage {
	#[must_use]
	pub fn to_eip712_bytes(&self, chain_id: u64, gateway_contract: Address) -> [u8; 66] {
		let domain_separator = {
			let mut bytes = [0u8; 148];
			bytes[0..32].copy_from_slice(&keccak_256(b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"));
			bytes[32..64].copy_from_slice(&keccak_256(b"Analog Gateway Contract"));
			bytes[64..96].copy_from_slice(&keccak_256(b"0.1.0"));
			bytes[96..128].copy_from_slice(&U256::from(chain_id).to_le_bytes::<32>());
			bytes[128..148].copy_from_slice(gateway_contract.as_ref());
			keccak_256(&bytes)
		};
		let payload_hash = {
			let mut bytes = [0u8; 212];
			bytes[0..32].copy_from_slice(&keccak_256(b"GmpPayload(bytes32 source,uint128 srcNetwork,address dest,uint128 destNetwork,uint256 gasLimit,uint256 salt,bytes data)"));
			bytes[32..64].copy_from_slice(self.payload.source.as_ref());
			bytes[64..80].copy_from_slice(&self.payload.srcNetwork.to_le_bytes());
			bytes[80..100].copy_from_slice(self.payload.dest.as_ref());
			bytes[100..116].copy_from_slice(&self.payload.destNetwork.to_le_bytes());
			bytes[116..148].copy_from_slice(&self.payload.gasLimit.to_le_bytes::<32>());
			bytes[148..180].copy_from_slice(&self.payload.salt.to_le_bytes::<32>());
			bytes[180..212].copy_from_slice(&keccak_256(&self.payload.data));
			keccak_256(&bytes)
		};
		let message_hash = {
			let mut bytes = [0u8; 68];
			bytes[0..32]
				.copy_from_slice(&keccak_256(b"GmpMessage(uint32 nonce,GmpPayload payload)"));
			bytes[32..36].copy_from_slice(&self.nonce.to_le_bytes());
			bytes[36..68].copy_from_slice(&payload_hash);
			keccak_256(&bytes)
		};
		let gmp_eip712_message = {
			let mut bytes = [0u8; 66];
			bytes[0..2].copy_from_slice(&[0x19u8, 0x01u8]);
			bytes[2..34].copy_from_slice(&domain_separator);
			bytes[34..66].copy_from_slice(&message_hash);
			bytes
		};
		gmp_eip712_message
	}

	#[must_use]
	pub fn to_eip712_typed_hash(&self, chain_id: u64, gateway_contract: Address) -> [u8; 32] {
		keccak_256(self.to_eip712_bytes(chain_id, gateway_contract).as_ref())
	}
}

impl From<WrappedGmpPayload> for GmpPayload {
	fn from(wrapped_msg: WrappedGmpPayload) -> Self {
		Self {
			source: wrapped_msg.source.into(),
			srcNetwork: wrapped_msg.src_network.into(),
			dest: wrapped_msg.dest.into(),
			destNetwork: wrapped_msg.dest_network.into(),
			gasLimit: wrapped_msg.gas_limit,
			salt: wrapped_msg.salt,
			data: wrapped_msg.data,
		}
	}
}

impl From<WrappedGmpMessage> for GmpMessage {
	fn from(wrapped_msg: WrappedGmpMessage) -> Self {
		Self {
			nonce: wrapped_msg.nonce.into(),
			payload: wrapped_msg.payload.into(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedGmpMessage {
	pub nonce: u32,
	pub payload: WrappedGmpPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedGmpPayload {
	pub source: [u8; 32],
	pub src_network: ChainId,
	pub dest: [u8; 20],
	pub dest_network: ChainId,
	pub gas_limit: U256,
	pub salt: U256,
	pub data: Vec<u8>,
}

pub fn split_tss_sig(signature: TssSignature) -> (U256, U256) {
	let e_bytes = &signature[0..32];
	let s_bytes = &signature[32..64];

	let e_bigint = U256::from_be_slice(e_bytes.into());
	let s_bigint = U256::from_be_slice(s_bytes.into());
	(e_bigint, s_bigint)
}
