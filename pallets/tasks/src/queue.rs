use crate::{Config, TaskPhaseState, Tasks};
use codec::{Codec, EncodeLike};
use core::marker::PhantomData;
use frame_support::storage::{StorageDoubleMap, StorageMap};
use time_primitives::{NetworkId, TaskId, TaskPhase};

pub trait TaskQ<T: Config> {
	/// Return the next `n` assignable tasks
	fn get_n(&self, n: usize, shard_size: u16, is_registered: bool) -> Vec<(u64, TaskId)>;
	/// Remove an item from the queue
	fn remove(&mut self, network: NetworkId, index: u64, task_id: TaskId);
	/// Push an item onto the end of the queue.
	fn push(&mut self, network: NetworkId, task_id: TaskId);
	/// Pop an item from the start of the queue.
	///
	/// Returns `None` if the queue is empty.
	fn pop(&mut self) -> Option<TaskId>;
	/// Return whether the queue is empty.
	fn is_empty(&self) -> bool;
}

pub struct TaskQueue<InsertIndex, RemoveIndex, Queue>
where
	InsertIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	RemoveIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	Queue: StorageDoubleMap<NetworkId, u64, TaskId, Query = Option<TaskId>>,
{
	// required for commit on Drop semantics
	network: NetworkId,
	insert: u64,
	remove: u64,
	_phantom: PhantomData<(InsertIndex, RemoveIndex, Queue)>,
}

impl<InsertIndex, RemoveIndex, Queue> TaskQueue<InsertIndex, RemoveIndex, Queue>
where
	InsertIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	RemoveIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	Queue: StorageDoubleMap<NetworkId, u64, TaskId, Query = Option<TaskId>>,
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
	InsertIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	RemoveIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	Queue: StorageDoubleMap<NetworkId, u64, TaskId, Query = Option<TaskId>>,
{
	fn get_n(&self, n: usize, shard_size: u16, is_registered: bool) -> Vec<(u64, TaskId)> {
		(self.remove..self.insert)
			.filter_map(|index| {
				Queue::get(self.network, index).and_then(|task_id| {
					Tasks::<T>::get(task_id)
						.filter(|task| {
							task.shard_size == shard_size
								&& (is_registered
									|| TaskPhaseState::<T>::get(task_id) != TaskPhase::Sign)
						})
						.map(|_| (index, task_id))
				})
			})
			.take(n)
			.collect::<Vec<_>>()
	}
	/// Remove an item from the queue
	fn remove(&mut self, network: NetworkId, index: u64, task_id: TaskId) {
		todo!()
	}
	/// Push an item onto the end of the queue.
	fn push(&mut self, network: NetworkId, task_id: TaskId) {
		todo!()
	}
	/// Pop an item from the start of the queue.
	///
	/// Returns `None` if the queue is empty.
	fn pop(&mut self) -> Option<TaskId> {
		todo!()
	}
	/// Return whether the queue is empty.
	fn is_empty(&self) -> bool {
		todo!()
	}
}
