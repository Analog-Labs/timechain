use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};

type GmpID = [u8; 32];
type NetworkId = u16;
type Address = [u8; 32];

pub const MAX_SHARDS_LEN: usize = 50;

#[derive(Accounts)]
pub struct Initialize<'info> {
	#[account(
        init,
        payer = signer,
        // first 8 bytes is type discriminator: <https://www.anchor-lang.com/docs/basics/program-structure#account-discriminator>
        space = 8 + 32 + 1,
        // look into the security issues of seeds
        seeds = [b"gateway_state"],
        bump
    )]
	pub gateway_state: Account<'info, GatewayState>,
	#[account(mut)]
	pub signer: Signer<'info>,

	pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetAdmin<'info> {
	#[account(mut)]
	pub gateway_state: Account<'info, GatewayState>,
	#[account(mut)]
	pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetShards<'info> {
	#[account(mut)]
	pub gateway_state: Account<'info, GatewayState>,
	#[account(mut)]
	pub signer: Signer<'info>,
}

#[account]
#[derive(Default)]
pub struct GatewayState {
	pub admin: Pubkey,
	pub is_initialized: bool,
	pub shards: Vec<ShardAcc>,
}

#[account]
pub struct GmpInfo {
	pub msg: [u8; 32],
	pub y_parity: u8,
	pub nonce: u64,
}

#[account]
pub struct ShardAcc {
	pub shard: Shard,
	pub nonce: u64,
}

#[account]
pub struct ShardNonce {
	pub nonce: u64,
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct Shard {
	pub x_coord: [u8; 32],
	pub y_parity: u8,
}

#[derive(Clone, Copy, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum GmpStatus {
	Pending,
	Executed,
	Failed,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct GmpMessage {
	pub src_network: NetworkId,
	pub dest_network: NetworkId,
	pub src: Address,
	pub dest: Address,
	pub nonce: u64,
	pub gas_limit: u128,
	pub gas_cost: u128,
	pub bytes: Vec<u8>,
}
