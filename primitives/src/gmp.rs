use crate::{Function, TssSignature};
use alloy_sol_types::SolCall;
use codec::alloc::string::String;
use sp_std::vec::Vec;
use IGateway::*;

pub fn make_register_shard_call(
	shard_public_key: [u8; 33],
	contract_address: [u8; 20],
) -> Function {
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
pub fn maybe_sign_register_shard(input: Vec<String>, sig: TssSignature) -> Vec<String> {
	let mut signed = Vec::new();
	for arg in input {
		let try_decode_register_shard =
			registerTSSKeysCall::abi_decode(&arg.clone().into_bytes(), false);
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
			signed.push(arg.into());
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
		uint8 parity;
		uint256 px;
		uint256 e;
		uint256 s;
	}

	// impl From<crate::TssSignature> for Signature {
	// 	fn from(signature: TssSignature) -> Signature {
	// 		todo!()
	// 	}
	// }

	#[derive(Debug, PartialEq, Eq)]
	struct TssKey {
		uint8 parity;    // public key y-coord parity (27 or 28)
		bytes32 coordX;  // public key x-coord
	}

	// impl From<[u8; 33]> for TssKey {
	// 	fn from(key: [u8; 33]) -> TssKey {
	// 		let mut key_vec = shard_public_key.to_vec();
	// 		let x_coordinate: [u8; 32] = shard_public_key[1..].try_into().unwrap();
	// 		let (parity, coordX) = (key_vec.remove(0), x_coordinate.into());
	// 		TssKey { parity, coordX }
	// 	}
	// }

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
		uint128 nonce;
		uint128 networkId; // source network id
		bytes32 sender;    // sender public key
		address dest;      // dest contract
		bytes payload;     // message payload
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
