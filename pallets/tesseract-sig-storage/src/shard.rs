//! Shard type utilities
use crate::{Config, Error};
use frame_support::ensure;
use sp_runtime::DispatchError;
use sp_std::{borrow::ToOwned, vec::Vec};
use time_primitives::{sharding::Shard, TimeId};

pub fn new_shard<T: Config>(
	members: Vec<TimeId>,
	collector_index: Option<u8>,
) -> Result<Shard, DispatchError> {
	let mut s = Shard::try_from(members).map_err(|_| Error::<T>::UnsupportedMembershipSize)?;
	if let Some(c) = collector_index {
		try_set_collector::<T>(&mut s, true, c)?;
	}
	Ok(s)
}

/// Sets collector to a given ID if is not the same.
/// Returns error if `collector` is not member of this shard.
/// # Param
/// * init - if the method is being called to initialize the shard
/// * index - is the current index of the collector
fn try_set_collector<T: Config>(
	shard: &mut Shard,
	init: bool,
	index: u8,
) -> Result<(), DispatchError> {
	if init && index == 0 {
		// collector already set
		return Ok(());
	}
	ensure!(index != 0, Error::<T>::AlreadyCollector);
	let set = &mut shard.members()[..];
	let new_collector = set.get(index as usize).ok_or(Error::<T>::CollectorIndexBeyondMemberLen)?;
	let old_collector = set[0].clone();
	set[0] = new_collector.to_owned();
	set[index as usize] = old_collector;
	Ok(())
}
