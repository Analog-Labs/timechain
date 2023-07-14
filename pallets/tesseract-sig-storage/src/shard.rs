//! Shard type utilities
use crate::{Config, Error, Event, Pallet, TssShards};
use codec::{Decode, Encode};
use frame_support::traits::Get;
use sp_runtime::{traits::Saturating, DispatchError};
use sp_std::{borrow::ToOwned, vec::Vec};
use time_primitives::{
	sharding::{Network, ReassignShardTasks, Shard},
	TimeId,
};

#[derive(Copy, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq)]
pub enum ShardStatus {
	Online,
	Offline,
}

#[derive(Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct ShardState {
	/// Shard membership
	pub shard: Shard,
	/// Number of tasks timed out by the shard collector
	pub task_timeout_count: u8,
	/// Number of committed offenses for the shard
	pub committed_offenses_count: u8,
	/// Status for shard
	pub status: ShardStatus,
	/// Network from which tasks assigned to this shard
	pub net: Network,
}

impl ShardState {
	pub fn new<T: Config>(
		members: Vec<TimeId>,
		collector_index: Option<u8>,
		net: Network,
	) -> Result<ShardState, DispatchError> {
		Ok(ShardState {
			shard: new_shard::<T>(members, collector_index)?,
			task_timeout_count: 0u8,
			committed_offenses_count: 0u8,
			status: ShardStatus::Offline,
			net,
		})
	}
	pub fn is_online(&self) -> bool {
		self.status == ShardStatus::Online
	}
	pub fn increment_task_timeout_count<T: Config>(&mut self, id: u64) {
		self.task_timeout_count = self.task_timeout_count.saturating_plus_one();
		let timeouts_above_max = self.task_timeout_count > T::MaxTimeouts::get();
		if timeouts_above_max && self.is_online() {
			// set shard to offline if cannot reach consensus and status is not offline
			self.status = ShardStatus::Offline;
			// reassign all of this shard's tasks to other shards
			T::TaskAssigner::reassign_shard_tasks(id);
			Pallet::<T>::deposit_event(Event::ShardOffline(id));
		}
		<TssShards<T>>::insert(id, self);
	}
	pub fn increment_committed_offense_count<T: Config>(&mut self, id: u64) {
		self.committed_offenses_count = self.committed_offenses_count.saturating_plus_one();
		let shard_cannot_reach_consensus = self.committed_offenses_count
			> (self.shard.members().len() as u8).saturating_sub(self.shard.threshold() as u8);
		if shard_cannot_reach_consensus && self.is_online() {
			// set shard to offline if cannot reach consensus and status is not offline
			self.status = ShardStatus::Offline;
			// reassign all of this shard's tasks to other shards
			T::TaskAssigner::reassign_shard_tasks(id);
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
