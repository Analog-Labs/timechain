use core::marker::PhantomData;
use parity_scale_codec::{Decode, Encode};
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;
#[cfg(feature = "std")]
use std::cell::RefCell;
#[cfg(feature = "std")]
use std::collections::BTreeMap;
#[cfg(feature = "std")]
use std::rc::Rc;

pub trait KV: Default + Clone {
	fn get(&self, key: &[u8]) -> Option<Vec<u8>>;

	fn insert(&self, key: &[u8], value: &[u8]);

	fn remove(&self, key: &[u8]);

	fn next_key(&self, key: &[u8]) -> Option<Vec<u8>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SpStorage;

impl KV for SpStorage {
	fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
		Some(sp_io::storage::get(&key)?.into())
	}

	fn insert(&self, key: &[u8], value: &[u8]) {
		sp_io::storage::set(key, value);
	}

	fn remove(&self, key: &[u8]) {
		sp_io::storage::clear(key);
	}

	fn next_key(&self, key: &[u8]) -> Option<Vec<u8>> {
		sp_io::storage::next_key(key)
	}
}

#[cfg(feature = "std")]
#[derive(Clone, Debug, Default)]
pub struct StdStorage(Rc<RefCell<BTreeMap<Vec<u8>, Vec<u8>>>>);

#[cfg(feature = "std")]
impl KV for StdStorage {
	fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
		self.0.borrow().get(key).cloned()
	}

	fn insert(&self, key: &[u8], value: &[u8]) {
		self.0.borrow_mut().insert(key.to_vec(), value.to_vec());
	}

	fn remove(&self, key: &[u8]) {
		self.0.borrow_mut().remove(key);
	}

	fn next_key(&self, key: &[u8]) -> Option<Vec<u8>> {
		self.0.borrow().keys().find(|k| k.as_slice() > key).cloned()
	}
}

fn prefix_key<P: Encode, K: Encode>(prefix: &P, key: &K) -> Vec<u8> {
	(prefix, key).encode()
}

#[derive(Clone, Default)]
pub struct Storage<S: KV>(S);

impl<S: KV> Storage<S> {
	pub fn get<P: Encode, K: Encode, V: Decode>(&self, prefix: &P, key: &K) -> Option<V> {
		let key = prefix_key(prefix, key);
		let bytes = self.0.get(&key)?;
		Decode::decode(&mut &bytes[..]).ok()
	}

	pub fn insert<P: Encode, K: Encode, V: Encode>(&self, prefix: &P, key: &K, value: &V) {
		let key = prefix_key(prefix, key);
		let value = value.encode();
		self.0.insert(&key, &value);
	}

	pub fn remove<P: Encode, K: Encode>(&self, prefix: &P, key: &K) {
		let key = prefix_key(prefix, key);
		self.0.remove(&key);
	}

	fn raw_keys<P: Encode>(&self, prefix: &P) -> RawKeyIterator<S> {
		RawKeyIterator::new(self.clone(), prefix.encode())
	}

	pub fn keys<P: Encode, K: Decode>(&self, prefix: &P) -> KeyIterator<S, K> {
		KeyIterator::new(self.clone(), self.raw_keys(prefix))
	}

	pub fn values<P: Encode, V: Decode>(&self, prefix: &P) -> ValueIterator<S, V> {
		ValueIterator::new(self.clone(), self.raw_keys(prefix))
	}

	pub fn iter<P: Encode, K: Decode, V: Decode>(&self, prefix: &P) -> StorageIterator<S, K, V> {
		StorageIterator::new(self.clone(), self.raw_keys(prefix))
	}
}

struct RawKeyIterator<S: KV> {
	storage: Storage<S>,
	prefix: Vec<u8>,
	last_key: Vec<u8>,
}

impl<S: KV> RawKeyIterator<S> {
	fn new(storage: Storage<S>, prefix: Vec<u8>) -> Self {
		Self {
			storage,
			last_key: prefix.clone(),
			prefix,
		}
	}
}

impl<S: KV> Iterator for RawKeyIterator<S> {
	type Item = Vec<u8>;

	fn next(&mut self) -> Option<Vec<u8>> {
		let next_key = self.storage.0.next_key(&self.last_key)?;
		if !next_key.starts_with(&self.prefix) {
			return None;
		}
		self.last_key = next_key.clone();
		Some(next_key)
	}
}

pub struct KeyIterator<S: KV, K: Decode> {
	_marker: PhantomData<K>,
	storage: Storage<S>,
	keys: RawKeyIterator<S>,
}

impl<S: KV, K: Decode> KeyIterator<S, K> {
	fn new(storage: Storage<S>, keys: RawKeyIterator<S>) -> Self {
		Self {
			_marker: PhantomData,
			storage,
			keys,
		}
	}
}

impl<S: KV, K: Decode> Iterator for KeyIterator<S, K> {
	type Item = K;

	fn next(&mut self) -> Option<Self::Item> {
		let key = self.keys.next()?;
		Decode::decode(&mut &key[..]).ok()
	}
}

pub struct ValueIterator<S: KV, V: Decode> {
	_marker: PhantomData<V>,
	storage: Storage<S>,
	keys: RawKeyIterator<S>,
}

impl<S: KV, V: Decode> ValueIterator<S, V> {
	fn new(storage: Storage<S>, keys: RawKeyIterator<S>) -> Self {
		Self {
			_marker: PhantomData,
			storage,
			keys,
		}
	}
}

impl<S: KV, V: Decode> Iterator for ValueIterator<S, V> {
	type Item = V;

	fn next(&mut self) -> Option<Self::Item> {
		let key = self.keys.next()?;
		let value = self.storage.0.get(&key)?;
		Decode::decode(&mut &value[..]).ok()
	}
}

pub struct StorageIterator<S: KV, K: Decode, V: Decode> {
	_marker: PhantomData<(K, V)>,
	storage: Storage<S>,
	keys: RawKeyIterator<S>,
}

impl<S: KV, K: Decode, V: Decode> StorageIterator<S, K, V> {
	fn new(storage: Storage<S>, keys: RawKeyIterator<S>) -> Self {
		Self {
			_marker: PhantomData,
			storage,
			keys,
		}
	}
}

impl<S: KV, K: Decode, V: Decode> Iterator for StorageIterator<S, K, V> {
	type Item = (K, V);

	fn next(&mut self) -> Option<Self::Item> {
		let key = self.keys.next()?;
		let value = self.storage.0.get(&key)?;
		let key = Decode::decode(&mut &key[..]).ok()?;
		let value = Decode::decode(&mut &value[..]).ok()?;
		Some((key, value))
	}
}
