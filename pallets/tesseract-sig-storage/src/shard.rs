//! Shard type
use crate::{Config, Error};
use codec::{Decode, Encode};
use frame_support::ensure;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::DispatchError;
use sp_std::{borrow::ToOwned, vec::Vec};
use time_primitives::TimeId;

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
	pub fn new<T: crate::Config>(
		members: Vec<TimeId>,
		collector_index: Option<u8>,
	) -> Result<Self, DispatchError> {
		let mut s = Shard::try_from(members).map_err(|_| Error::<T>::UnsupportedMembershipSize)?;
		if let Some(c) = collector_index {
			s.try_set_collector::<T>(c)?;
		}
		Ok(s)
	}
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
	/// * index - is the current index of the collector
	pub fn try_set_collector<T: Config>(&mut self, index: u8) -> Result<(), DispatchError> {
		ensure!(index != 0, Error::<T>::AlreadyCollector);
		let set = self.inner();
		let new_collector =
			set.get(index as usize).ok_or(Error::<T>::CollectorIndexBeyondMemberLen)?;
		let old_collector = set[0].clone();
		set[0] = new_collector.to_owned();
		set[index as usize] = old_collector;
		Ok(())
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
	type Error = ();
	fn try_from(set: Vec<TimeId>) -> Result<Shard, ()> {
		match set.len() {
			3 => Ok(Shard::Three(arrayref::array_ref!(set, 0, 3).to_owned())),
			5 => Ok(Shard::Five(arrayref::array_ref!(set, 0, 5).to_owned())),
			10 => Ok(Shard::Ten(arrayref::array_ref!(set, 0, 10).to_owned())),
			_ => Err(()),
		}
	}
}
