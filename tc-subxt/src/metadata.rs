/// Helper macro to map derived metadata
#[macro_export]
macro_rules! metadata_scope {
	( $variant:expr, $block:block ) => {
		match $variant {
			$crate::metadata::MetadataVariant::Mainnet => {
				use tc_subxt_metadata::timechain as metadata;

				#[allow(unused)]
				use metadata::runtime_types::mainnet_runtime::RuntimeCall;

				#[allow(unused)]
				fn sudo(call: RuntimeCall) -> impl subxt::tx::Payload {
					use scale_codec::Encode;
					let length = call.encoded_size() as u32;
					metadata::tx().technical_committee().execute(call, length)
				}

				$block
			},
			$crate::metadata::MetadataVariant::Staging => {
				use tc_subxt_metadata::staging as metadata;

				#[allow(unused)]
				use metadata::runtime_types::mainnet_runtime::RuntimeCall;

				#[allow(unused)]
				fn sudo(call: RuntimeCall) -> impl subxt::tx::Payload {
					use scale_codec::Encode;
					let length = call.encoded_size() as u32;
					metadata::tx().technical_committee().execute(call, length)
				}

				$block
			},
			$crate::metadata::MetadataVariant::Testnet => {
				use tc_subxt_metadata::testnet as metadata;

				#[allow(unused)]
				use metadata::runtime_types::testnet_runtime::RuntimeCall;

				#[allow(unused)]
				fn sudo(call: RuntimeCall) -> impl subxt::tx::Payload {
					metadata::tx().sudo().sudo(call)
				}

				$block
			},
			$crate::metadata::MetadataVariant::Development => {
				use tc_subxt_metadata::development as metadata;

				#[allow(unused)]
				use metadata::runtime_types::testnet_runtime::RuntimeCall;

				#[allow(unused)]
				fn sudo(call: RuntimeCall) -> impl subxt::tx::Payload {
					metadata::tx().sudo().sudo(call)
				}

				$block
			},
		}
	};
}

/// Specifies the targeted timechain variant and metadata
#[derive(clap::ValueEnum, Clone, Copy, Default, Debug)]
pub enum MetadataVariant {
	Mainnet,
	Staging,
	#[default]
	Testnet,
	Development,
}
