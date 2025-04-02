use anchor_lang::prelude::*;
pub const MAX_SHARDS_LEN: usize = 50;

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

#[account]
#[derive(InitSpace)]
pub struct GatewayState {
	pub admin: Pubkey,
	pub is_initialized: bool,
	#[max_len(MAX_SHARDS_LEN)]
	pub shards: Vec<ShardAcc>,
}

#[account]
#[derive(InitSpace)]
pub struct ShardAcc {
	pub shard: Shard,
	pub nonce: u64,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub struct Shard {
	pub x_coord: [u8; 32],
	pub y_parity: u8,
}
