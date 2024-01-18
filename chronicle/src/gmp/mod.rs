pub mod types;
use alloy_primitives::{b256, Address, U256};
use alloy_sol_types::SolCall;
use time_primitives::{ChainId, Function, ShardId, TssPublicKey, TssSignature};
use types::{Eip712Ext, GmpMessage, SignableMessage, Signature, TssKey, UpdateKeysMessage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageBuilder {
	pub shard_id: ShardId,
	pub chain_id: u64,
	pub tss_public_key: TssKey,
	pub gateway_contract: Address,
}

impl MessageBuilder {
	pub fn new(
		shard_id: ShardId,
		chain_id: ChainId,
		tss_public_key: TssPublicKey,
		gateway_contract: [u8; 20],
	) -> Self {
		Self {
			shard_id,
			chain_id,
			tss_public_key: TssKey::from(tss_public_key),
			gateway_contract: Address(gateway_contract.into()),
		}
	}

	pub fn build_update_keys_message<REGISTER, REVOKE>(
		&self,
		revoke: REVOKE,
		register: REGISTER,
	) -> Message
	where
		REGISTER: IntoIterator<Item = TssPublicKey>,
		REVOKE: IntoIterator<Item = TssPublicKey>,
	{
		let message = UpdateKeysMessage {
			revoke: revoke.into_iter().map(TssKey::from).collect(),
			register: register.into_iter().map(TssKey::from).collect(),
		};
		Message {
			message: TypedMessage::UpdateKeys(message),
			chain_id: self.chain_id,
			tss_public_key: self.tss_public_key.clone(),
			gateway_contract: self.gateway_contract,
		}
	}

	pub fn build_gmp_message(
		&self,
		address: [u8; 20],
		payload: Vec<u8>,
		salt: [u8; 32],
		gas_limit: u64,
	) -> Message {
		let message = GmpMessage {
			// TODO: receive the sender address and network as parameter
			source: b256!("0000000000000000000000000000000000000000000000000000000000000000"),
			// TODO: fix this during cross chain communication
			srcNetwork: u128::from(self.chain_id),
			dest: Address(address.into()),
			destNetwork: u128::from(self.chain_id),
			gasLimit: U256::from(gas_limit),
			salt: U256::from_be_bytes(salt),
			data: payload,
		};
		Message {
			message: TypedMessage::Gmp(message),
			chain_id: self.chain_id,
			tss_public_key: self.tss_public_key.clone(),
			gateway_contract: self.gateway_contract,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TypedMessage {
	UpdateKeys(UpdateKeysMessage),
	Gmp(GmpMessage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
	chain_id: u64,
	gateway_contract: Address,
	tss_public_key: TssKey,
	message: TypedMessage,
}

impl Message {
	pub fn hash(&self) -> Vec<u8> {
		match &self.message {
			TypedMessage::UpdateKeys(message) => {
				message.to_eip712_bytes(self.chain_id, self.gateway_contract).into()
			},
			TypedMessage::Gmp(message) => {
				message.to_eip712_bytes(self.chain_id, self.gateway_contract).into()
			},
		}
	}

	/// compute the sighash of the message
	pub fn _sighash(&self) -> [u8; 32] {
		match &self.message {
			TypedMessage::UpdateKeys(message) => {
				message.to_eip712_typed_hash(self.chain_id, self.gateway_contract)
			},
			TypedMessage::Gmp(message) => {
				message.to_eip712_typed_hash(self.chain_id, self.gateway_contract)
			},
		}
	}

	/// Converts `Message` into `Function::EvmCall`
	pub fn into_evm_call(self, signature: TssSignature) -> Function {
		let signature = Signature {
			xCoord: self.tss_public_key.xCoord,
			e: U256::from_be_slice(&signature[0..32]),
			s: U256::from_be_slice(&signature[32..64]),
		};
		let payload = match self.message {
			TypedMessage::UpdateKeys(message) => message.into_call(signature).abi_encode(),
			TypedMessage::Gmp(message) => message.into_call(signature).abi_encode(),
		};
		Function::EvmCall {
			address: self.gateway_contract.into(),
			input: payload,
			amount: 0u128,
		}
	}
}
