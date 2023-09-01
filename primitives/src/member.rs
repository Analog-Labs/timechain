use crate::{Network, PeerId, PublicKey};
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[cfg(feature = "std")]
use sp_api::ApiError;

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
pub trait SubmitMembers<B: sp_runtime::traits::Block> {
	fn submit_register_member(
		&self,
		block: B::Hash,
		network: Network,
		public_key: PublicKey,
		peer_id: PeerId,
	) -> Result<(), ApiError>;

	fn submit_heartbeat(&self, block: B::Hash, public_key: PublicKey) -> Result<(), ApiError>;
}
