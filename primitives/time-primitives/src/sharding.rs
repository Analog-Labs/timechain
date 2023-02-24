use super::TimeId;
use anyhow::{Error, Result};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::{borrow::ToOwned, vec::Vec};

pub const FILTER_KEY_BYTES: [u8; 64] = [
	36, 30, 30, 34, 34, 31, 65, 35, 34, 39, 39, 36, 61, 66, 63, 36, 32, 32, 38, 32, 35, 65, 63, 31,
	31, 36, 65, 64, 62, 64, 31, 34, 35, 61, 36, 36, 32, 61, 61, 31, 38, 62, 66, 39, 36, 38, 64, 61,
	38, 62, 36, 34, 30, 34, 66, 35, 61, 36, 64, 39, 36, 36, 35, 30,
];

pub const FILTER_PALLET_KEY_BYTES: [u8; 32] = [
	66, 30, 63, 33, 36, 35, 63, 33, 63, 66, 35, 39, 64, 36, 37, 31, 65, 62, 37, 32, 64, 61, 30, 65,
	37, 61, 34, 31, 31, 33, 63, 34,
];

pub const FILTER_STORAGE_KEY_BYTES: [u8; 32] = [
	39, 66, 31, 66, 30, 35, 31, 35, 66, 34, 36, 32, 63, 64, 63, 66, 38, 34, 65, 30, 66, 31, 64, 36,
	30, 34, 35, 64, 66, 63, 62, 62,
];

/// Enum representing sizes of shards available
/// Each shard holds accounts of it's members
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum Shard {
	Three([TimeId; 3]),
	Five([TimeId; 5]),
	Ten([TimeId; 10]),
}

impl Shard {
	/// Returns ref to current collector ID
	/// First node in array (index 0) considered to be collector
	pub fn collector(&self) -> &TimeId {
		match self {
			Shard::Three(set) => &set[0],
			Shard::Five(set) => &set[0],
			Shard::Ten(set) => &set[0],
		}
	}

	/// Sets collector to a given ID if is not the same.
	/// Returns error if `collector` is not member of this shard.
	/// # Param
	/// * collector - ref to `TimeId`, which is expected to be collector for this shard
	pub fn set_collector(&mut self, collector: &TimeId) -> Result<()> {
		if self.collector() == collector {
			return Ok(());
		}
		let set = self.inner();
		if let Some(index) = set.iter().position(|member| member == collector) {
			let old = set[0].clone();
			set[0] = collector.to_owned();
			set[index] = old;
			Ok(())
		} else {
			Err(Error::msg("Given collector id is not a member of current shard."))
		}
	}

	/// Returns owned Vec<TimeId> of current shard members
	pub fn members(&self) -> Vec<TimeId> {
		match self {
			Shard::Three(set) => set.to_vec(),
			Shard::Five(set) => set.to_vec(),
			Shard::Ten(set) => set.to_vec(),
		}
	}

	// intentionaly private
	fn inner(&mut self) -> &mut [TimeId] {
		match self {
			Shard::Three(set) => set,
			Shard::Five(set) => set,
			Shard::Ten(set) => set,
		}
	}
}

impl TryFrom<Vec<TimeId>> for Shard {
	type Error = Error;
	fn try_from(set: Vec<TimeId>) -> Result<Shard> {
		match set.len() {
			3 => Ok(Shard::Three(arrayref::array_ref!(set, 0, 3).to_owned())),
			5 => Ok(Shard::Five(arrayref::array_ref!(set, 0, 5).to_owned())),
			10 => Ok(Shard::Ten(arrayref::array_ref!(set, 0, 10).to_owned())),
			_ => Err(Error::msg("wrong number of nodes. supported sizes are 3, 5 or 10")),
		}
	}
}
