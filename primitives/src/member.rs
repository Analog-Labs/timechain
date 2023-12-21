#[cfg(feature = "std")]
use crate::{AccountId, ApiResult, BlockHash, SubmitResult};
use crate::{Network, PeerId, PublicKey};
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Copy, Debug, Encode, Decode, TypeInfo)]
pub struct HeartbeatInfo<BlockNumber> {
	pub is_online: bool,
	pub block: BlockNumber,
}

impl<B: Copy> HeartbeatInfo<B> {
	pub fn new(block: B) -> Self {
		Self { is_online: true, block }
	}

	pub fn set_offline(&self) -> Self {
		Self {
			is_online: false,
			block: self.block,
		}
	}
}

#[cfg(feature = "std")]
pub trait Members {
	fn get_member_peer_id(
		&self,
		block: BlockHash,
		account: &AccountId,
	) -> ApiResult<Option<PeerId>>;

	fn get_heartbeat_timeout(&self) -> ApiResult<u64>;

	fn get_min_stake(&self) -> ApiResult<u128>;

	fn submit_register_member(
		&self,
		network: Network,
		peer_id: PeerId,
		stake_amount: u128,
	) -> SubmitResult;

	fn submit_heartbeat(&self) -> SubmitResult;
}

#[cfg(feature = "std")]
pub trait MembersPayload {
	fn submit_register_member(
		&self,
		network: Network,
		public_key: PublicKey,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Vec<u8>;
	fn submit_heartbeat(&self) -> Vec<u8>;
}
