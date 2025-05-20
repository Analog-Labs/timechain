use anchor_client::anchor_lang::prelude::*;
use anchor_client::{
	anchor_lang::{AnchorDeserialize, AnchorSerialize},
	solana_sdk::pubkey::Pubkey,
};
use time_primitives::NetworkId;

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
	pub routes: Vec<NetworkInfo>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct NetworkInfo {
	network_id: u16,
	destination_gateway: Pubkey,
	relative_gas_price_n: u128,
	relative_gas_price_d: u128,
	gas_limit: u64,
	gmp_base_fee: u128,
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

impl From<time_primitives::Route> for NetworkInfo {
	fn from(value: time_primitives::Route) -> Self {
		Self {
			network_id: value.network_id,
			destination_gateway: a_addr(value.gateway),
			// FIXME wrong conversion from u256 to u128
			relative_gas_price_n: value.relative_gas_price.0.as_u128(),
			// FIXME wrong conversion from u256 to u128
			relative_gas_price_d: value.relative_gas_price.1.as_u128(),
			gas_limit: value.gas_limit,
			gmp_base_fee: value.gmp_base_fee,
		}
	}
}

impl From<NetworkInfo> for time_primitives::Route {
	fn from(value: NetworkInfo) -> Self {
		Self {
			network_id: value.network_id,
			gateway: t_addr(value.destination_gateway),
			relative_gas_price: (
				// FIXME fix take u256 instead of u128
				value.relative_gas_price_n.into(),
				value.relative_gas_price_d.into(),
			),
			gas_limit: value.gas_limit,
			gmp_base_fee: value.gmp_base_fee,
		}
	}
}
