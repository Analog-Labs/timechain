use crate::{NetworkId, PublicKey};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
	pub network: NetworkId,
	pub account: String,
	pub public_key: PublicKey,
	pub address: String,
	pub peer_id: String,
	pub peer_id_hex: String,
}
