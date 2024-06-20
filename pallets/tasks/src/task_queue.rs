use codec::{Codec, EncodeLike};
use core::marker::PhantomData;
use frame_support::storage::{StorageDoubleMap, StorageMap};
use time_primitives::{NetworkId, TaskId};

/// Task queue interface.
pub trait TaskQueueStorage<Item>
where
	Item: Codec + EncodeLike,
{
	/// Store all changes made in the underlying storage.
	///
	/// Data is not guaranteed to be consistent before this call.
	///
	/// Implementation note: Call in `drop` to increase ergonomics.
	fn commit(&self);
	/// Take up to `count` number of items from the queue
	fn take(&mut self, network: NetworkId, count: usize) -> Vec<Item>;
	/// Remove an item from the queue
	fn remove(&mut self, network: NetworkId, index: u64, i: Item);
	/// Push an item onto the end of the queue.
	fn push(&mut self, network: NetworkId, i: Item);
	/// Pop an item from the start of the queue.
	///
	/// Returns `None` if the queue is empty.
	fn pop(&mut self) -> Option<Item>;
	/// Return whether the queue is empty.
	fn is_empty(&self) -> bool;
}

/// Transient backing data that is the backbone of the trait object.
pub struct TaskQueue<Index, Queue>
where
	Index: StorageMap<NetworkId, u64, Query = u64>,
	Queue: StorageDoubleMap<NetworkId, u64, TaskId, Query = TaskId>,
{
	insert: u64,
	remove: u64,
	_phantom: PhantomData<(Index, Queue)>,
}
