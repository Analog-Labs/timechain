//! Shard type utilities
use crate::{Config, Error};
use sp_runtime::DispatchError;
use sp_std::{borrow::ToOwned, vec::Vec};
use time_primitives::{sharding::Shard, TimeId};

pub fn new_shard<T: Config>(
	members: Vec<TimeId>,
	collector_index: Option<u8>,
) -> Result<Shard, DispatchError> {
	let shard = if let Some(index) = collector_index {
		if index == 0 {
			members
		} else {
			let mut set = members;
			let new_collector =
				set.get(index as usize).ok_or(Error::<T>::CollectorIndexBeyondMemberLen)?;
			let old_collector = set[0].clone();
			set[0] = new_collector.to_owned();
			set[index as usize] = old_collector;
			set
		}
	} else {
		members
	};
	Ok(Shard::try_from(shard).map_err(|_| Error::<T>::UnsupportedMembershipSize)?)
}
