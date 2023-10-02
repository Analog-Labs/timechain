use crate::Network;
use scale_info::prelude::string::String;

#[derive(Clone)]
pub struct WalletParams {
	pub blockchain: Network,
	pub network: String,
	pub url: String,
	pub keyfile: Option<String>,
}
