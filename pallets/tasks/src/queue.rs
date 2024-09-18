use crate::Config;
use core::marker::PhantomData;
use frame_support::storage::{StorageDoubleMap, StorageMap};
use polkadot_sdk::{frame_support, sp_runtime};
use scale_codec::FullCodec;
use sp_runtime::Saturating;
use time_primitives::NetworkId;

pub type Index = u64;

pub trait QueueT<T: Config, Value> {
	/// Push an item onto the end of the queue.
	fn push(&self, value: Value);
	/// Remove an item from the queue.
	fn remove(&self, index: Index) -> Option<Value>;
	/// Pop an item from the beginning of the queue.
	fn pop(&self) -> Option<Value>;
}

pub struct QueueImpl<T, Value, InsertIndex, RemoveIndex, Queue> {
	network: NetworkId,
	_phantom: PhantomData<(T, Value, InsertIndex, RemoveIndex, Queue)>,
}

impl<T, Value, InsertIndex, RemoveIndex, Queue>
	QueueImpl<T, Value, InsertIndex, RemoveIndex, Queue>
{
	pub fn new(network: NetworkId) -> QueueImpl<T, Value, InsertIndex, RemoveIndex, Queue> {
		Self { network, _phantom: PhantomData }
	}
}

impl<T: Config, Value, InsertIndex, RemoveIndex, Queue> QueueT<T, Value>
	for QueueImpl<T, Value, InsertIndex, RemoveIndex, Queue>
where
	Value: FullCodec,
	InsertIndex: StorageMap<NetworkId, Index, Query = Option<Index>>,
	RemoveIndex: StorageMap<NetworkId, Index, Query = Option<Index>>,
	Queue: StorageDoubleMap<NetworkId, Index, Value, Query = Option<Value>>,
{
	fn pop(&self) -> Option<Value> {
		let remove_i = RemoveIndex::get(self.network).unwrap_or_default();
		self.remove(remove_i)
	}

	fn push(&self, value: Value) {
		let insert_i = InsertIndex::get(self.network).unwrap_or_default();
		Queue::insert(self.network, insert_i, value);
		InsertIndex::insert(self.network, insert_i.saturating_plus_one());
	}

	fn remove(&self, index: Index) -> Option<Value> {
		let insert_i = InsertIndex::get(self.network).unwrap_or_default();
		let mut remove_i = RemoveIndex::get(self.network).unwrap_or_default();
		if remove_i >= insert_i {
			return None;
		}
		let value = Queue::take(self.network, index);
		if index == remove_i {
			remove_i = remove_i.saturating_plus_one();
			while Queue::get(self.network, remove_i).is_none() && remove_i < insert_i {
				remove_i = remove_i.saturating_plus_one();
			}
			RemoveIndex::insert(self.network, remove_i);
		}
		value
	}
}
