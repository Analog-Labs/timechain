use anchor_lang::prelude::*;
use gmp_solana_contract::{
	GatewayState, GmpMessage,
	cpi::{SubmitMessage, submit_message},
};

declare_id!("11111111111111111111111111111111");

#[program]
pub mod example_contract {
	use super::*;
	pub fn send(
		ctx: Context<SendMessage>,
		dest_network: u16,
		dest: [u8; 32],
		gas_limit: u64,
		payload: Vec<u8>,
	) -> Result<[u8; 32]> {
		let msg = GmpMessage {
			src_network: 1,
			dest_network,
			src: ctx.accounts.authority.key().to_bytes(),
			dest,
			gas_limit,
			bytes: payload,
			nonce: 1,
		};

		let cpi_ctx = CpiContext::new(
			ctx.accounts.gateway_program.to_account_info(),
			SubmitMessage {
				gateway_state: ctx.accounts.gateway_state.clone(),
				signer: ctx.accounts.authority.clone(),
			},
		);

		submit_message(cpi_ctx, msg)
	}
	pub fn receive(
		ctx: Context<ReceiveMessage>,
		id: [u8; 32],
		src_network: u16,
		src: [u8; 32],
		nonce: u64,
		payload: Vec<u8>,
	) -> Result<[u8; 32]> {
		let state = &mut ctx.accounts.receiver_state;
		state.last_message_id = id;

		emit!(MessageReceived {
			id,
			src_network,
			src,
			nonce,
			payload,
		});

		Ok(id)
	}
}

#[derive(Accounts)]
pub struct SendMessage<'info> {
	#[account(mut)]
	pub gateway_state: Account<'info, GatewayState>,
	#[account(mut)]
	pub authority: Signer<'info>,
	/// CHECK: Gateway program
	pub gateway_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct ReceiveMessage<'info> {
	#[account(mut)]
	pub receiver_state: Account<'info, ReceiverState>,
}

#[account]
pub struct ReceiverState {
	pub last_message_id: [u8; 32],
}

#[event]
pub struct MessageReceived {
	pub id: [u8; 32],
	pub src_network: u16,
	pub src: [u8; 32],
	pub nonce: u64,
	pub payload: Vec<u8>,
}
