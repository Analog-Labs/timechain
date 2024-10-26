use crate::NetworkId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
	pub network: NetworkId,
	pub account: String,
	pub address: String,
	pub peer_id: String,
}

impl Config {
	#[must_use]
	pub const fn new(network: NetworkId, account: String, address: String, peer_id: String) -> Self {
		Self {
			network,
			account,
			address,
			peer_id,
		}
	}
}
