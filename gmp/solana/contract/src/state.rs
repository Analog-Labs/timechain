// #![allow(unexpected_cfgs)]
// use crate::borsh::maybestd::collections::HashMap;
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::keccak;

use crate::GatewayError;

type NetworkId = u16;
type Address32 = [u8; 32];
pub type BatchId = u64;
pub type MessageId = [u8; 32];
pub type TssPublicKey = [u8; 33];
pub const MAX_SHARDS_LEN: usize = 50;
pub const MAX_NETWORKS_LEN: usize = 50;

#[derive(Accounts)]
pub struct Initialize<'info> {
	#[account(
        init,
        payer = signer,
        space = 8 + std::mem::size_of::<Gateway>(),
        seeds = [&GmpPdaSeeds::State.to_seed()],
        bump
    )]
	pub gateway_state: Account<'info, GatewayState>,
	#[account(mut)]
	pub signer: Signer<'info>,
	pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Gateway<'info> {
	#[account(mut,
	    seeds = [&GmpPdaSeeds::State.to_seed()],
	    bump
	)]
	pub gateway_state: Account<'info, GatewayState>,
	#[account(mut)]
	pub signer: Signer<'info>,
}

#[account]
pub struct GmpMessageState {
	pub admin: MessageId,
	pub status: GmpStatus,
}

#[account]
pub struct GmpInfo {
	pub msg: [u8; 32],
	pub y_parity: u8,
	pub nonce: u64,
}

#[account]
#[derive(InitSpace)]
pub struct GatewayState {
	pub admin: Pubkey,
	pub is_initialized: bool,
	#[max_len(MAX_SHARDS_LEN)]
	pub shards: Vec<Shard>,
	#[max_len(MAX_NETWORKS_LEN)]
	pub routes: Vec<Route>,
}

#[account]
#[derive(InitSpace)]
pub struct Route {
	pub network_id: u16,
	pub gateway: Pubkey,
	pub max_gas_limit: u64,
	pub msg_gas: u64,
	pub msg_byte_gas: u64,
	pub gas_price: f64,
	pub msg_fee: u64,
}

#[account]
pub struct ShardNonce {
	pub nonce: u64,
}

#[event]
pub struct GmpCreated {
	pub msg_id: MessageId,
	pub msg: GmpMessage,
}

#[event]
pub struct GmpExecuted {
	pub msg_id: MessageId,
}

#[event]
pub struct BatchExecuted {
	pub batch_id: BatchId,
}

#[event]
pub struct ShardRevoked {
	pub x_coord: [u8; 32],
	pub y_parity: u8,
	pub num_sessions: u16,
}
#[event]
pub struct ShardRegistered {
	pub x_coord: [u8; 32],
	pub y_parity: u8,
	pub num_sessions: u16,
}

#[derive(Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub enum GmpStatus {
	Pending,
	Executed,
	Failed,
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct GmpMessage {
	pub src_network: NetworkId,
	pub dest_network: NetworkId,
	pub src: Address32,
	pub dest: Address32,
	pub nonce: u64,
	pub gas_limit: u128,
	pub gas_cost: u128,
	pub bytes: Vec<u8>,
}

impl GmpMessage {
	const HEADER_LEN: usize = 224;

	pub fn encoded_len(&self) -> usize {
		Self::HEADER_LEN + self.bytes.len()
	}

	fn encode_header(&self) -> [u8; 224] {
		let mut hdr = [0u8; 224];
		hdr[32..64].copy_from_slice(&left_pad(&self.src));
		hdr[64..96].copy_from_slice(&left_pad(&self.src_network.to_be_bytes()));
		hdr[96..128].copy_from_slice(&left_pad(&self.dest));
		hdr[128..160].copy_from_slice(&left_pad(&self.dest_network.to_be_bytes()));
		hdr[160..192].copy_from_slice(&left_pad(&self.gas_limit.to_be_bytes()));
		hdr[192..224].copy_from_slice(&left_pad(&self.nonce.to_be_bytes()));
		hdr
	}

	pub fn message_id(&self) -> MessageId {
		let header = self.encode_header();
		keccak::hash(&header).0
	}
}

fn left_pad(data: &[u8]) -> [u8; 32] {
	assert!(data.len() <= 32, "data is too long to pad to 32 bytes");
	let mut padded = [0u8; 32];
	let offset = 32 - data.len();
	padded[offset..].copy_from_slice(data);
	padded
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct GatewayMessage {
	pub ops: Vec<GatewayOp>,
}
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum GatewayOp {
	SendMessage(GmpMessage),
	RegisterShard(TssPublicKey),
	UnregisterShard(TssPublicKey),
}
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

#[derive(Clone, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub struct Shard {
	pub x_coord: [u8; 32],
	pub y_parity: u8,
	pub num_sessions: u16,
}

impl Shard {
	pub fn from_tss_key(tss_key: &TssPublicKey, num_sessions: u16) -> Result<Self> {
		let y_parity = if tss_key[0] == 0x02 {
			27
		} else if tss_key[0] == 0x03 {
			28
		} else {
			return Err(GatewayError::InvalidYParity.into());
		};

		let mut x_coord = [0u8; 32];
		x_coord.copy_from_slice(&tss_key[1..33]);

		Ok(Shard {
			x_coord,
			y_parity,
			num_sessions,
		})
	}

	pub fn to_tss_key(&self) -> TssPublicKey {
		let mut tss_key = [0u8; 33];
		tss_key[0] = if self.y_parity == 27 { 0x02 } else { 0x03 };
		tss_key[1..33].copy_from_slice(&self.x_coord);
		tss_key
	}
}
