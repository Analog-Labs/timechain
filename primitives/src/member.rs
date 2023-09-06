#[cfg(feature = "std")]
use crate::SubmitResult;
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
pub trait SubmitMembers {
	fn submit_register_member(
		&self,
		network: Network,
		public_key: PublicKey,
		peer_id: PeerId,
	) -> SubmitResult;

	fn submit_heartbeat(&self, public_key: PublicKey) -> SubmitResult;
}
