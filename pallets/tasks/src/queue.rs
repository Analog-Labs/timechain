use codec::{Codec, EncodeLike};
use core::marker::PhantomData;
use frame_support::storage::{StorageDoubleMap, StorageMap};
use time_primitives::{NetworkId, TaskId};

pub trait TaskQ {
	/// Store all changes made in the underlying storage. Always commit on Drop.
	fn commit(&self);
	/// Take up to `count` number of items from the queue
	fn take(&mut self, network: NetworkId, count: usize) -> Vec<TaskId>;
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

impl<InsertIndex, RemoveIndex, Queue> Drop for TaskQueue<InsertIndex, RemoveIndex, Queue>
where
	InsertIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	RemoveIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	Queue: StorageDoubleMap<NetworkId, u64, TaskId, Query = Option<TaskId>>,
{
	/// Commit on `drop`.
	fn drop(&mut self) {
		<Self as TaskQ>::commit(self);
	}
}

impl<InsertIndex, RemoveIndex, Queue> TaskQ for TaskQueue<InsertIndex, RemoveIndex, Queue>
where
	InsertIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	RemoveIndex: StorageMap<NetworkId, u64, Query = Option<u64>>,
	Queue: StorageDoubleMap<NetworkId, u64, TaskId, Query = Option<TaskId>>,
{
	/// Store all changes made in the underlying storage
	// TODO: determine if commit on Drop semantics is helpful or if we should just push changes to storage in the other helper functions.
	fn commit(&self) {
		InsertIndex::insert(self.network, self.insert);
		RemoveIndex::insert(self.network, self.remove);
	}
	/// Take up to `count` number of items from the queue
	fn take(&mut self, network: NetworkId, count: usize) -> Vec<TaskId> {
		todo!()
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
