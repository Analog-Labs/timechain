use crate::{GatewayState, GmpMessage, GmpPdaSeeds};
use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_program::{instruction::Instruction, program::invoke};

#[derive(Accounts)]
pub struct SubmitMessage<'info> {
	#[account(mut, seeds = [&GmpPdaSeeds::State.to_seed()], bump)]
	pub gateway_state: Account<'info, GatewayState>,
	#[account(mut)]
	pub signer: Signer<'info>,
}

pub fn submit_message<'a, 'b, 'c, 'info>(
	ctx: CpiContext<'a, 'b, 'c, 'info, SubmitMessage<'info>>,
	msg: GmpMessage,
) -> Result<[u8; 32]> {
	let instruction = crate::instruction::SubmitMessage { msg: msg.clone() };

	let accounts = vec![
		AccountMeta::new(ctx.accounts.gateway_state.key(), false),
		AccountMeta::new(*ctx.accounts.signer.key, true),
	];

	let instruction = Instruction {
		program_id: crate::ID,
		accounts,
		data: instruction.data(),
	};

	invoke(
		&instruction,
		&[ctx.accounts.gateway_state.to_account_info(), ctx.accounts.signer.to_account_info()],
	)?;

	Ok(msg.message_id())
}
