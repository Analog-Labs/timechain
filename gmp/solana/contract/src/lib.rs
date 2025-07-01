#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

mod constants;
mod errors;
mod state;

use constants::*;
use errors::*;
pub use state::*;

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
	pub fn set_admin(ctx: Context<Gateway>, new_admin: Pubkey) -> Result<()> {
		let state = &mut ctx.accounts.gateway_state;
		require_keys_eq!(ctx.accounts.signer.key(), state.admin, GatewayError::Unauthorized);
		require!(state.is_initialized, GatewayError::AlreadyInitialized);
		state.admin = new_admin;
		Ok(())
	}

	// only executed by admin
	pub fn set_shards(
		ctx: Context<Gateway>,
		register: Vec<(TssPublicKey, u16)>, // (public_key, num_sessions)
		revoke: Vec<TssPublicKey>,
	) -> Result<()> {
		let state = &mut ctx.accounts.gateway_state;
		require_keys_eq!(ctx.accounts.signer.key(), state.admin, GatewayError::Unauthorized);
		require!(state.is_initialized, GatewayError::NotInitialized);
		for (tss_key, num_sessions) in register {
			let shard = Shard::from_tss_key(&tss_key, num_sessions)?;
			require!(shard.y_parity == 27 || shard.y_parity == 28, GatewayError::InvalidYParity);
			let existing_index = state.shards.iter().position(|s| s.x_coord == shard.x_coord);
			if existing_index.is_none() {
				require!(
					state.shards.len() < MAX_SHARDS_LEN,
					GatewayError::ShardsLengthExceedLimit
				);
				state.shards.push(shard.clone());
				emit!(ShardRegistered { shard: shard.clone() });
			}

			for tss_key in &revoke {
				let shard = Shard::from_tss_key(&tss_key, 0)?;

				if let Some(index) = state.shards.iter().position(|s| s.x_coord == shard.x_coord) {
					require!(
						state.shards[index].y_parity == shard.y_parity,
						GatewayError::YParityMismatch
					);
					let removed_shard = state.shards.remove(index);
					emit!(ShardRevoked { shard: removed_shard });
				}
			}
		}
		Ok(())
	}

	pub fn set_route(ctx: Context<Gateway>, route: Route) -> Result<()> {
		let state = &mut ctx.accounts.gateway_state;
		require_keys_eq!(ctx.accounts.signer.key(), state.admin, GatewayError::Unauthorized);
		require!(state.is_initialized, GatewayError::NotInitialized);

		require!(route.max_gas_limit > 0, GatewayError::InvalidGasLimit);
		require!(route.gas_price > 0.0, GatewayError::InvalidGasPrice);

		if let Some(existing) = state.routes.iter_mut().find(|r| r.network_id == route.network_id) {
			*existing = route.clone();
			emit!(RouteUpdated { route: route.clone() });
		} else {
			require!(state.routes.len() < MAX_NETWORKS_LEN, GatewayError::RoutesLengthExceedLimit);
			state.routes.push(route.clone());
			emit!(RouteAdded { route });
		}

		Ok(())
	}

	// excuted by user
	pub fn submit_message(_ctx: Context<Gateway>, msg: GmpMessage) -> Result<()> {
		require_gt!(MAX_PAYLOAD_SIZE, msg.bytes.len() as u128, GatewayError::MsgTooLarge);
		let msg_id = msg.message_id();
		emit!(GmpCreated { msg_id, msg });
		Ok(())
	}

	// excuted by chronicles
	pub fn execute_batch(
		ctx: Context<Gateway>,
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
