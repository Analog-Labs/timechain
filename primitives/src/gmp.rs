use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

#[derive(Clone, Debug, Encode, Decode, TypeInfo)]
pub struct Gateway {
	pub address: Vec<u8>,
	pub chain_id: u64,
}
