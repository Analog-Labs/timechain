#![cfg_attr(not(feature = "std"), no_std)]

use crate::storage::{SpStorage, Storage, KV};
use parity_scale_codec::{Decode, Encode};
use time_primitives::{ChainName, ChainNetwork, NetworkId};

mod storage;

#[derive(Clone, Copy, Debug, Encode, Decode)]
enum Prefix {
	Networks,
	NetworkId,
}

#[derive(Clone, Default)]
pub struct Networks<S: KV>(Storage<S>);

impl Networks<SpStorage> {
	pub fn new() -> Self {
		Self::default()
	}
}

impl<S: KV> Networks<S> {
	pub fn get(&self, id: NetworkId) -> Option<(ChainName, ChainNetwork)> {
		self.0.get(&Prefix::Networks, &id)
	}

	pub fn insert(&self, name: ChainName, network: ChainNetwork) -> NetworkId {
		for (id, (chain_name, chain_network)) in
			self.0.iter::<_, NetworkId, (ChainName, ChainNetwork)>(&Prefix::Networks)
		{
			if chain_name == name && chain_network == network {
				return id;
			}
		}
		let id: NetworkId = self.0.get(&Prefix::NetworkId, b"").unwrap_or_default();
		self.0.insert(&Prefix::Networks, &id, &(name, network));
		self.0.insert(&Prefix::NetworkId, b"", &(id + 1));
		id
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::storage::StdStorage;

	#[test]
	fn test_networks() {
		let eth = ChainName::from("eth");
		let sep = ChainNetwork::from("sepolia");
		let networks = Networks::<StdStorage>::default();
		assert!(networks.get(0).is_none());
		networks.insert(eth.clone(), sep.clone());
		assert_eq!(networks.get(0).unwrap(), (eth.clone(), sep.clone()));
		networks.insert(eth.clone(), sep.clone());
		assert!(networks.get(1).is_none());
		assert_eq!(networks.get(0).unwrap(), (eth, sep));
	}
}
