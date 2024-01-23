#[cfg(feature = "std")]
use crate::ApiResult;
use scale_info::prelude::string::String;

pub type NetworkId = u64;
pub type ChainName = String;
pub type ChainNetwork = String;

#[cfg(feature = "std")]
pub trait Networks {
	fn get_network(&self, network: NetworkId) -> ApiResult<Option<(String, String)>>;
}
