use anchor_client::anchor_lang::prelude::*;
use anchor_client::{
	anchor_lang::{AnchorDeserialize, AnchorSerialize},
	solana_sdk::pubkey::Pubkey,
};

use crate::{a_addr, t_addr};

pub enum GmpPdaSeeds {
	State,
	Vault,
}

impl GmpPdaSeeds {
	pub fn to_seed(&self) -> Vec<u8> {
		match self {
			GmpPdaSeeds::State => b"gateway_state".into(),
			GmpPdaSeeds::Vault => b"gateway_vault".into(),
		}
	}
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GatewayState {
	pub admin: Pubkey,
	pub is_initialized: bool,
	pub shards: Vec<ShardAcc>,
	pub routes: Vec<Route>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Route {
	pub network_id: u16,
	pub gateway: Pubkey,
	pub max_gas_limit: u64,
	pub msg_gas: u64,
	pub msg_byte_gas: u64,
	pub gas_price: f64,
	pub msg_fee: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ShardAcc {
	pub shard: Shard,
	pub nonce: u64,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Shard {
	pub x_coord: [u8; 32],
	pub y_parity: u8,
}

// Conversion functions
impl From<Shard> for time_primitives::TssPublicKey {
	fn from(value: Shard) -> Self {
		let mut shard_key = [0u8; 33];
		shard_key[0] = value.y_parity;
		shard_key[1..33].copy_from_slice(&value.x_coord);
		shard_key
	}
}

impl From<time_primitives::TssPublicKey> for Shard {
	fn from(value: time_primitives::TssPublicKey) -> Self {
		let mut x_coord = [0u8; 32];
		x_coord.copy_from_slice(&value[1..33]);
		Self { x_coord, y_parity: value[0] }
	}
}

impl From<time_primitives::Route> for Route {
	fn from(value: time_primitives::Route) -> Self {
		Self {
			network_id: value.network_id,
			gateway: a_addr(value.gateway),
			max_gas_limit: value.max_gas_limit,
			msg_gas: value.msg_gas,
			msg_byte_gas: value.msg_byte_gas,
			gas_price: value.gas_price,
			msg_fee: value.msg_fee,
		}
	}
}

impl From<Route> for time_primitives::Route {
	fn from(value: Route) -> Self {
		Self {
			network_id: value.network_id,
			gateway: t_addr(value.gateway),
			max_gas_limit: value.max_gas_limit,
			msg_gas: value.msg_gas,
			msg_byte_gas: value.msg_byte_gas,
			gas_price: value.gas_price,
			msg_fee: value.msg_fee,
		}
	}
}
