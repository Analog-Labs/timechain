use crate::Config;
use core::marker::PhantomData;
use frame_support::storage::{StorageDoubleMap, StorageMap};
use polkadot_sdk::{frame_support, sp_runtime, sp_std};
use scale_codec::FullCodec;
use sp_runtime::Saturating;
use sp_std::vec::Vec;
use time_primitives::NetworkId;

pub type Index = u64;

pub trait QueueT<T: Config, Value> {
	/// Return the next `n` assignable tasks.
	fn get_n(&self, n: usize) -> Vec<(Index, Value)>;
	/// Push an item onto the end of the queue.
	fn push(&self, value: Value);
	/// Remove an item from the queue.
	fn remove(&mut self, index: Index);
}

pub struct QueueImpl<Value, InsertIndex, RemoveIndex, Queue> {
	network: NetworkId,
	insert: Index,
	remove: Index,
	_phantom: PhantomData<(Value, InsertIndex, RemoveIndex, Queue)>,
}

impl<Value, InsertIndex, RemoveIndex, Queue> QueueImpl<Value, InsertIndex, RemoveIndex, Queue>
where
	Value: FullCodec,
	InsertIndex: StorageMap<NetworkId, Index, Query = Option<Index>>,
	RemoveIndex: StorageMap<NetworkId, Index, Query = Option<Index>>,
	Queue: StorageDoubleMap<NetworkId, Index, Value, Query = Option<Value>>,
{
	pub fn new(network: NetworkId) -> QueueImpl<Value, InsertIndex, RemoveIndex, Queue> {
		let insert = InsertIndex::get(network).unwrap_or_default();
		let remove = RemoveIndex::get(network).unwrap_or_default();
		Self {
			network,
			insert,
			remove,
			_phantom: PhantomData,
		}
	}
}

impl<T: Config, Value, InsertIndex, RemoveIndex, Queue> QueueT<T, Value>
	for QueueImpl<Value, InsertIndex, RemoveIndex, Queue>
where
	Value: FullCodec,
	InsertIndex: StorageMap<NetworkId, Index, Query = Option<Index>>,
	RemoveIndex: StorageMap<NetworkId, Index, Query = Option<Index>>,
	Queue: StorageDoubleMap<NetworkId, Index, Value, Query = Option<Value>>,
{
	fn get_n(&self, n: usize) -> Vec<(Index, Value)> {
		(self.remove..self.insert)
			.filter_map(|index| Queue::get(self.network, index).map(|value| (index, value)))
			.take(n)
			.collect::<Vec<_>>()
	}

	fn push(&self, value: Value) {
		Queue::insert(self.network, self.insert, value);
		InsertIndex::insert(self.network, self.insert.saturating_plus_one());
	}

	fn remove(&mut self, index: Index) {
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
