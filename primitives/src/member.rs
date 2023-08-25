use crate::Network;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub struct MemberState {
    pub network: Network,
    pub status: MemberStatus,
}

impl MemberState {
    pub fn new(network: Network) -> MemberState {
        MemberState {
            network,
            status: MemberStatus::Unassigned,
        }
    }
}

#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum MemberStatus {
	Assigned,
	Unassigned,
}

impl MemberStatus {
    pub fn assigned(&self) -> bool {
        matches!(self, MemberStatus::Assigned)
    }
}