pub mod types;
use alloy_primitives::{address, b256, hex, Address, U256};
use types::{Eip712Ext, SignableMessage, GmpMessage, TssKey, UpdateKeysMessage};
use time_primitives::{
	BlockHash, Network, ShardId, TaskExecution, TaskPhase, Tasks, TssId, Shards, TssPublicKey
};

pub struct GmpBuilder {
	pub shard_id: ShardId,
	pub chain_id: u64,
	pub tss_public_key: TssKey,
	pub gateway_contract: Address,
}

impl GmpBuilder {
	pub fn new(shard_id: ShardId, network: Network, tss_public_key: TssPublicKey, gateway_contract: [u8; 20]) -> Self {
		Self {
			shard_id,
			chain_id: network.eip155_chain_id(),
			tss_public_key: TssKey::from(tss_public_key),
			gateway_contract: Address(gateway_contract.into()),
		}
	}

	pub fn build_update_keys_message<REGISTER, REVOKE>(&self, register: REGISTER, revoke: REVOKE) -> UpdateKeysMessage where REGISTER: IntoIterator<Item = TssPublicKey>, REVOKE: IntoIterator<Item = TssPublicKey> {
		UpdateKeysMessage {
			register: register.into_iter().map(TssKey::from).collect(),
			revoke: revoke.into_iter().map(TssKey::from).collect(),
		}
	}

	pub fn compute_register_sighash(&self, tss_public_key: TssPublicKey) -> [u8; 32] {
		let message = self.build_update_keys_message([tss_public_key], []);
		message.to_eip712_typed_hash(self.chain_id, self.gateway_contract)
	}

	pub fn compute_revoke_sighash(&self, tss_public_key: TssPublicKey) -> [u8; 32] {
		let message = self.build_update_keys_message([], [tss_public_key]);
		message.to_eip712_typed_hash(self.chain_id, self.gateway_contract)
	}

	pub fn build_gmp_message(&self, address: [u8;20], payload: Vec<u8>, salt: [u8;32], gas_limit: u64) -> GmpMessage {
		GmpMessage {
			// TODO: receive the sender address and network as parameter
			source: b256!("0000000000000000000000000000000000000000000000000000000000000000"),
			srcNetwork: 1337,
			dest: Address(address.into()),
			destNetwork: u128::from(self.chain_id),
			gasLimit: U256::from(gas_limit),
			salt: U256::from_be_bytes(salt),
			data: payload,
		}
	}

	pub fn compute_gmp_sighash(&self, address: [u8;20], payload: Vec<u8>, salt: [u8;32], gas_limit: u64) -> [u8; 32] {
		let message = self.build_gmp_message(address, payload, salt, gas_limit);
		message.to_eip712_typed_hash(self.chain_id, self.gateway_contract)
	}
}
