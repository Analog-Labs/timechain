use crate::AccountId;
use scale_codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};

pub type DmailTo = String;
pub type DmailPath = String;

#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone, Debug)]
pub struct DmailMessage {
	pub owner: AccountId,
	pub to: DmailTo,
	pub path: DmailPath,
}
