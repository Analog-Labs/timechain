use crate::{ChainId, Function, TssSignature};
pub use alloy_primitives::U256;
use alloy_sol_types::SolCall;
use codec::alloc::string::String;
use ethabi::Token;
use scale_info::prelude::vec::Vec;
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use sp_core::hashing::keccak_256;
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
	struct GMPPayload {
		bytes32 source;      // Pubkey/Address of who send the GMP message
		uint128 srcNetwork;  // Source chain identifier (it's the EIP-155 chain_id for ethereum networks)
		address dest;        // Destination/Recipient contract address
		uint128 destNetwork; // Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
		uint256 gasLimit;    // gas limit of the GMP call
		uint256 salt;        // Message salt, useful for sending two messages with same content
		bytes data;          // message data with no specified format
	}

	#[derive(Debug, PartialEq, Eq)]
	struct GMPMessage{
		uint256 nonce;
		GMPPayload payload;
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
		function sudoExecute(GMPPayload memory message) external returns (bool success);
		function execute(Signature memory signature, GMPMessage memory message) external returns (bool success);
	}
}

impl From<WrappedGmpPayload> for GMPPayload {
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

#[cfg(feature = "std")]
pub fn get_gmp_msg_hash(payload: WrappedGmpPayload, contract_address: Vec<u8>) -> [u8; 32] {
	let chain_id = payload.src_network;
	let mut contract_arr = [0u8; 20];
	contract_arr.copy_from_slice(&contract_address);
	let domain_seperator = compute_domain_separator(chain_id, contract_arr);
	[0; 32]
}

#[cfg(feature = "std")]
fn get_gmp_payload_hash(payload: WrappedGmpPayload) -> [u8; 32] {
	let encoded_payload_type = keccak_256(b"GMPPayload(bytes32 source,uint96 srcNetwork,address dest,uint96 destNetwork,bytes32 sender,uint256 gasLimit,uint256 value,uint256 salt,bytes data)");

	let encoded = ethabi::encode(&[
		Token::FixedBytes(encoded_payload_type.to_vec()),
		Token::FixedBytes(payload.source.to_vec()),
		Token::Uint(payload.src_network.into()),
		Token::FixedBytes(payload.dest.to_vec()),
		Token::Uint(payload.dest_network.into()),
		Token::Uint(sp_core::U256::from_big_endian(&payload.gas_limit.to_be_bytes_trimmed_vec())),
		Token::Uint(sp_core::U256::from_big_endian(&payload.salt.to_be_bytes_trimmed_vec())),
		Token::FixedBytes(keccak_256(&payload.data).to_vec()),
	]);

	keccak_256(&encoded).into()
}

#[cfg(feature = "std")]
fn compute_domain_separator(chain_id: u64, contract_address: [u8; 20]) -> [u8; 32] {
	let encoded = ethabi::encode(&[
		Token::FixedBytes(
			b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
				.to_vec(),
		),
		Token::FixedBytes(b"Analog Gateway Contract".to_vec()),
		Token::FixedBytes(b"0.1.0".to_vec()),
		Token::Uint(chain_id.into()),
		Token::Address(contract_address.into()),
	]);

	keccak_256(&encoded)
}
