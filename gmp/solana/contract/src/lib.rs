use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

#[program]
mod hello_anchor {
	use super::*;

	pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
		Ok(())
	}
}

#[derive(Accounts)]
pub struct Initialize<'info> {
	#[account(mut)]
	pub signer: Signer<'info>,
	pub system_program: Program<'info, System>,
}

#[account]
pub struct NewAccount {
	data: u64,
}
