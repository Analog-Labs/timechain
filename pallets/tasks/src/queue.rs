use crate::{Config, Tasks, UATaskIndex};
use core::marker::PhantomData;

use polkadot_sdk::{frame_support, sp_runtime, sp_std};

use frame_support::storage::{StorageDoubleMap, StorageMap};
use sp_runtime::Saturating;
use sp_std::vec::Vec;

use time_primitives::{NetworkId, TaskId, TaskIndex};

pub trait TaskQ<T: Config> {
	/// Return the next `n` assignable tasks.
	fn get_n(&self, n: usize) -> Vec<(u64, TaskId)>;
	/// Push an item onto the end of the queue.
	fn push(&self, task_id: TaskId);
	/// Remove an item from the queue.
	fn remove(&mut self, index: TaskIndex);
}

pub struct TaskQueue<InsertIndex, RemoveIndex, Queue>
where
	InsertIndex: StorageMap<NetworkId, TaskIndex, Query = Option<TaskIndex>>,
	RemoveIndex: StorageMap<NetworkId, TaskIndex, Query = Option<TaskIndex>>,
	Queue: StorageDoubleMap<NetworkId, TaskIndex, TaskId, Query = Option<TaskId>>,
{
	network: NetworkId,
	insert: TaskIndex,
	remove: TaskIndex,
	_phantom: PhantomData<(InsertIndex, RemoveIndex, Queue)>,
}

impl<InsertIndex, RemoveIndex, Queue> TaskQueue<InsertIndex, RemoveIndex, Queue>
where
	InsertIndex: StorageMap<NetworkId, TaskIndex, Query = Option<TaskIndex>>,
	RemoveIndex: StorageMap<NetworkId, TaskIndex, Query = Option<TaskIndex>>,
	Queue: StorageDoubleMap<NetworkId, TaskIndex, TaskId, Query = Option<TaskId>>,
{
	pub fn new(n: NetworkId) -> TaskQueue<InsertIndex, RemoveIndex, Queue> {
		let (insert, remove) = (InsertIndex::get(n).unwrap_or(0), RemoveIndex::get(n).unwrap_or(0));
		TaskQueue {
			network: n,
			insert,
			remove,
			_phantom: PhantomData,
		}
	}
}

impl<T: Config, InsertIndex, RemoveIndex, Queue> TaskQ<T>
	for TaskQueue<InsertIndex, RemoveIndex, Queue>
where
	InsertIndex: StorageMap<NetworkId, TaskIndex, Query = Option<TaskIndex>>,
	RemoveIndex: StorageMap<NetworkId, TaskIndex, Query = Option<TaskIndex>>,
	Queue: StorageDoubleMap<NetworkId, TaskIndex, TaskId, Query = Option<TaskId>>,
{
	fn get_n(&self, n: usize) -> Vec<(u64, TaskId)> {
		(self.remove..self.insert)
			.filter_map(|index| {
				Queue::get(self.network, index)
					.and_then(|task_id| Tasks::<T>::get(task_id).map(|_| (index, task_id)))
			})
			.take(n)
			.collect::<Vec<_>>()
	}
	fn push(&self, task_id: TaskId) {
		Queue::insert(self.network, self.insert, task_id);
		UATaskIndex::<T>::insert(task_id, self.insert);
		InsertIndex::insert(self.network, self.insert.saturating_plus_one());
	}
	fn remove(&mut self, index: TaskIndex) {
		if self.remove >= self.insert {
			return;
		}
		Queue::remove(self.network, index);
		if index == self.remove {
			self.remove = self.remove.saturating_plus_one();
			while Queue::get(self.network, self.remove).is_none() && self.remove < self.insert {
				self.remove = self.remove.saturating_plus_one();
			}
			RemoveIndex::insert(self.network, self.remove);
		}
	}
}
