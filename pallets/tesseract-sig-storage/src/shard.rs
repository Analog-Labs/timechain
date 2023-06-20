//! Shard type utilities
use crate::{Config, Error, Event, Pallet, TssShards};
use codec::{Decode, Encode};
use sp_runtime::{traits::Saturating, DispatchError};
use sp_std::{borrow::ToOwned, vec::Vec};
use time_primitives::{sharding::Shard, TimeId};

#[derive(Copy, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq)]
pub enum ShardStatus {
	Online,
	Offline,
}

#[derive(Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct ShardState {
	pub shard: Shard,
	pub committed_offenses_count: u8,
	pub status: ShardStatus,
}

impl ShardState {
	pub fn new<T: Config>(
		members: Vec<TimeId>,
		collector_index: Option<u8>,
	) -> Result<ShardState, DispatchError> {
		Ok(ShardState {
			shard: new_shard::<T>(members, collector_index)?,
			committed_offenses_count: 0u8,
			status: ShardStatus::Online,
		})
	}
	pub fn is_online(&self) -> bool {
		self.status == ShardStatus::Online
	}
	pub fn increment_committed_offense_count<T: Config>(&mut self, id: u64) {
		self.committed_offenses_count = self.committed_offenses_count.saturating_plus_one();
		let shard_cannot_reach_consensus = self.committed_offenses_count
			> (self.shard.members().len() as u8).saturating_sub(self.shard.threshold() as u8);
		if shard_cannot_reach_consensus && self.is_online() {
			// set shard to offline if cannot reach consensus and status is not offline
			self.status = ShardStatus::Offline;
			Pallet::<T>::deposit_event(Event::ShardOffline(id));
		}
		<TssShards<T>>::insert(id, self);
	}
}

fn new_shard<T: Config>(
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
