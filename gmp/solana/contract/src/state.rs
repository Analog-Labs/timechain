use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::keccak;

type NetworkId = u16;
type Address32 = [u8; 32];
pub type MessageId = [u8; 32];

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

#[derive(Accounts)]
pub struct SetRoute<'info> {
	#[account(mut)]
	pub gateway_state: Account<'info, GatewayState>,
	#[account(mut)]
	pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct SubmitMessage<'info> {
	#[account(mut)]
	pub gateway_state: Account<'info, GatewayState>,
	#[account(mut)]
	pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteBatch<'info> {
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

#[event]
pub struct GmpCreated {
	pub msg_id: MessageId,
	pub msg: GmpMessage,
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct NetworkInfo {
	gas_limit: u64,
	relative_gas_price: (u64, u64),
	base_fee: u128,
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
