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
