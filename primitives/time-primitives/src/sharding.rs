use super::TimeId;
use anyhow::{Error, Result};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::{borrow::ToOwned, vec::Vec};

pub const FILTER_PALLET_KEY_BYTES: [u8; 32] = [
	194, 38, 18, 118, 204, 157, 31, 133, 152, 234, 75, 106, 116, 177, 92, 47, 87, 200, 117, 228,
	207, 247, 65, 72, 228, 98, 143, 38, 75, 151, 76, 128,
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

	/// Returns the shard threshold.
	pub fn threshold(&self) -> u16 {
		match self {
			Shard::Three(_) => 2,
			Shard::Five(_) => 3,
			Shard::Ten(_) => 7,
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
