#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

mod constants;
mod errors;
mod state;

use constants::*;
use errors::*;
use state::*;

declare_id!("11111111111111111111111111111111");

#[program]
mod gateway {
	use super::*;
	// Admin executable
	pub fn initialize(ctx: Context<Initialize>, admin: Pubkey) -> Result<()> {
		let state = &mut ctx.accounts.gateway_state;
		if !state.is_initialized {
			state.admin = admin;
			state.is_initialized = true;
		}
		Ok(())
	}

	// Admin executable
	pub fn set_admin(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
		let state = &mut ctx.accounts.gateway_state;
		require_keys_eq!(ctx.accounts.signer.key(), state.admin, GatewayError::Unauthorized);
		require!(state.is_initialized, GatewayError::AlreadyInitialized);
		state.admin = new_admin;
		Ok(())
	}

	// only executed by admin
	pub fn set_shards(ctx: Context<SetShards>, shards: Vec<Shard>) -> Result<()> {
		require!(shards.len() < MAX_SHARDS_LEN, GatewayError::ShardsLengthExceedLimit);
		let state = &mut ctx.accounts.gateway_state;
		require_keys_eq!(ctx.accounts.signer.key(), state.admin, GatewayError::Unauthorized);
		state.shards.clear();
		for shard in shards.into_iter() {
			let seed_x = shard.x_coord;
			let seed_y = [shard.y_parity];

			let (nonce_pda, _bump) =
				Pubkey::find_program_address(&[b"shard_nonce", &seed_x, &seed_y], ctx.program_id);

			let mut shard_nonce = 0u64;
			let mut found = false;
			for acc in ctx.remaining_accounts.iter() {
				if acc.key() == nonce_pda {
					let shard_nonce_acc =
						ShardNonce::try_deserialize(&mut acc.data.borrow().as_ref())?;
					shard_nonce = shard_nonce_acc.nonce;
					found = true;
					break;
				}
			}
			if !found {
				shard_nonce = 0;
			}
			state.shards.push(ShardAcc {
				shard: shard.clone(),
				nonce: shard_nonce,
			});
		}

		Ok(())
	}

	pub fn set_route(ctx: Context<SetRoute>, _route: NetworkInfo) -> Result<()> {
		let state = &mut ctx.accounts.gateway_state;
		require_keys_eq!(ctx.accounts.signer.key(), state.admin, GatewayError::Unauthorized);
		Ok(())
	}

	// excuted by user
	pub fn submit_message(_ctx: Context<SubmitMessage>, msg: GmpMessage) -> Result<()> {
		require_gt!(MAX_PAYLOAD_SIZE, msg.bytes.len() as u128, GatewayError::MsgTooLarge);
		let msg_id = msg.message_id();
		emit!(GmpCreated { msg_id, msg });
		Ok(())
	}

	// excuted by chronicles
	pub fn execute_batch(
		ctx: Context<ExecuteBatch>,
		msg: GatewayMessage,
		batch_id: BatchId,
	) -> Result<()> {
		let state = &mut ctx.accounts.gateway_state;
		require_keys_eq!(ctx.accounts.signer.key(), state.admin, GatewayError::Unauthorized);
		// TODO verify signature etc
		for op in msg.ops.iter() {
			match op {
				GatewayOp::SendMessage(gmp_message) => {
					let msg_id = gmp_message.message_id();
					emit!(GmpExecuted { msg_id });
				},
				GatewayOp::RegisterShard(_) => {},
				GatewayOp::UnregisterShard(_) => {},
			}
		}
		emit!(BatchExecuted { batch_id });
		Ok(())
	}
}
