use crate::{ChainId, Function, TssSignature};
pub use alloy_primitives::U256;
use alloy_sol_types::SolCall;
use serde::{Deserialize, Serialize};
use sp_core::keccak_256;
use sp_std::vec::Vec;
use IGateway::*;

pub fn form_gmp_send_message(
	chain_id: U256,
	shard_nonce: u64,
	contract_address: [u8; 20],
	gmp: GmpPayload,
) -> Function {
	let domain_separator = keccak_256(
		[
			keccak_256(b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
			keccak_256(b"Analog Gateway Contract"),
			keccak_256(b"0.1.0"),
			chain_id.as_le_bytes(),
			contract_address.as_slice(),
		].concat()
	);

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
			keccak_256(gmp.data),
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
		contract_address: contract_address.to_vec(),
		payload: gmp_eip712_message,
	}
}

pub fn form_register_shard_send_message(
	chain_id: U256,
	shard_nonce: u64,
	contract_address: [u8; 20],
	shard_public_key: [u8; 33],
) -> Function {
	let gmp: GmpPayload = todo!();
	// sudoRegisterTSSKeysCall {
	// 	tssKeys: sp_std::vec![shard_public_key.into()],
	// }
	// .abi_encode()
	form_gmp_send_message(chain_id, shard_nonce, contract_address, gmp)
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
	struct GMPMessage {
		bytes32 source;      // Pubkey/Address of who send the GMP message
		uint128 srcNetwork;  // Source chain identifier (it's the EIP-155 chain_id for ethereum networks)
		address dest;        // Destination/Recipient contract address
		uint128 destNetwork; // Destination chain identifier (it's the EIP-155 chain_id for ethereum networks)
		uint256 gasLimit;    // gas limit of the GMP call
		uint256 salt;        // Message salt, useful for sending two messages with same content
		bytes data;          // message data with no specified format
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
		function sudoExecute(GMPMessage memory message) external returns (bool success);
		function execute(Signature memory signature, GMPMessage memory message) external returns (bool success);
	}
}

impl From<WrappedGmpMessage> for GMPMessage {
	fn from(wrapped_msg: WrappedGmpMessage) -> Self {
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
pub struct WrappedGmpMessage {
	pub source: [u8; 32],
	pub src_network: ChainId,
	pub dest: [u8; 20],
	pub dest_network: ChainId,
	pub gas_limit: U256,
	pub salt: U256,
	pub data: Vec<u8>,
}
