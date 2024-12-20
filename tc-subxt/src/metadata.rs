//use serde::{Deserialize, Serialize};

pub use runtime_types::timechain_runtime::RuntimeCall;
pub use tc_subxt_metadata::timechain::*;

pub fn sudo(call: RuntimeCall) -> impl subxt::tx::Payload {
	use scale_codec::Encode;
	let length = call.encoded_size() as u32;
	tx().technical_committee().execute(call, length)
}
