use crate::AccountId;
use polkadot_sdk::{sp_core::ConstU32, sp_runtime::BoundedVec};
use scale_codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};

pub const DMAIL_TO_LEN: u32 = 100;
pub const DMAIL_PATH_LEN: u32 = 100;

pub type DmailTo = BoundedVec<u8, ConstU32<DMAIL_TO_LEN>>;
pub type DmailPath = BoundedVec<u8, ConstU32<DMAIL_PATH_LEN>>;

#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone, Debug)]
pub struct DmailMessage {
	pub owner: AccountId,
	pub to: DmailTo,
	pub path: DmailPath,
}
