use scale_codec::{Decode, Encode};

#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/mainnet.default.scale",
	derive_for_all_types = "PartialEq, Clone",
	substitute_type(
		path = "time_primitives::shard::MemberStatus",
		with = "::subxt::utils::Static<::time_primitives::MemberStatus>",
	),
	substitute_type(
		path = "time_primitives::shard::ShardStatus",
		with = "::subxt::utils::Static<::time_primitives::ShardStatus>",
	),
	substitute_type(
		path = "time_primitives::task::Payload",
		with = "::subxt::utils::Static<::time_primitives::Payload>",
	),
	substitute_type(
		path = "time_primitives::task::TaskDescriptor",
		with = "::subxt::utils::Static<::time_primitives::TaskDescriptor>",
	),
	substitute_type(
		path = "time_primitives::task::TaskExecution",
		with = "::subxt::utils::Static<::time_primitives::TaskExecution>",
	),
	substitute_type(
		path = "time_primitives::task::TaskPhase",
		with = "::subxt::utils::Static<::time_primitives::TaskPhase>",
	),
	substitute_type(
		path = "sp_runtime::MultiSigner",
		with = "::subxt::utils::Static<crate::metadata::MultiSigner>",
	)
)]
pub mod timechain {}

#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/mainnet.development.scale",
	derive_for_all_types = "PartialEq, Clone",
	substitute_type(
		path = "time_primitives::shard::MemberStatus",
		with = "::subxt::utils::Static<::time_primitives::MemberStatus>",
	),
	substitute_type(
		path = "time_primitives::shard::ShardStatus",
		with = "::subxt::utils::Static<::time_primitives::ShardStatus>",
	),
	substitute_type(
		path = "time_primitives::task::Payload",
		with = "::subxt::utils::Static<::time_primitives::Payload>",
	),
	substitute_type(
		path = "time_primitives::task::TaskDescriptor",
		with = "::subxt::utils::Static<::time_primitives::TaskDescriptor>",
	),
	substitute_type(
		path = "time_primitives::task::TaskExecution",
		with = "::subxt::utils::Static<::time_primitives::TaskExecution>",
	),
	substitute_type(
		path = "time_primitives::task::TaskPhase",
		with = "::subxt::utils::Static<::time_primitives::TaskPhase>",
	),
	substitute_type(
		path = "sp_runtime::MultiSigner",
		with = "::subxt::utils::Static<crate::metadata::MultiSigner>",
	)
)]
pub mod staging {}

#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/testnet.default.scale",
	derive_for_all_types = "PartialEq, Clone",
	substitute_type(
		path = "time_primitives::shard::MemberStatus",
		with = "::subxt::utils::Static<::time_primitives::MemberStatus>",
	),
	substitute_type(
		path = "time_primitives::shard::ShardStatus",
		with = "::subxt::utils::Static<::time_primitives::ShardStatus>",
	),
	substitute_type(
		path = "time_primitives::task::Payload",
		with = "::subxt::utils::Static<::time_primitives::Payload>",
	),
	substitute_type(
		path = "time_primitives::task::TaskDescriptor",
		with = "::subxt::utils::Static<::time_primitives::TaskDescriptor>",
	),
	substitute_type(
		path = "time_primitives::task::TaskExecution",
		with = "::subxt::utils::Static<::time_primitives::TaskExecution>",
	),
	substitute_type(
		path = "time_primitives::task::TaskPhase",
		with = "::subxt::utils::Static<::time_primitives::TaskPhase>",
	),
	substitute_type(
		path = "sp_runtime::MultiSigner",
		with = "::subxt::utils::Static<crate::metadata::MultiSigner>",
	)
)]
pub mod testnet {}

#[subxt::subxt(
	runtime_metadata_path = "../config/subxt/testnet.development.scale",
	derive_for_all_types = "PartialEq, Clone",
	substitute_type(
		path = "time_primitives::shard::MemberStatus",
		with = "::subxt::utils::Static<::time_primitives::MemberStatus>",
	),
	substitute_type(
		path = "time_primitives::shard::ShardStatus",
		with = "::subxt::utils::Static<::time_primitives::ShardStatus>",
	),
	substitute_type(
		path = "time_primitives::task::Payload",
		with = "::subxt::utils::Static<::time_primitives::Payload>",
	),
	substitute_type(
		path = "time_primitives::task::TaskDescriptor",
		with = "::subxt::utils::Static<::time_primitives::TaskDescriptor>",
	),
	substitute_type(
		path = "time_primitives::task::TaskExecution",
		with = "::subxt::utils::Static<::time_primitives::TaskExecution>",
	),
	substitute_type(
		path = "time_primitives::task::TaskPhase",
		with = "::subxt::utils::Static<::time_primitives::TaskPhase>",
	),
	substitute_type(
		path = "sp_runtime::MultiSigner",
		with = "::subxt::utils::Static<crate::metadata::MultiSigner>",
	)
)]
pub mod development {}

/// Specifies the targeted timechain variant and metadata
#[derive(clap::ValueEnum, Clone, Copy, Default, Debug)]
pub enum Variant {
	Mainnet,
	Staging,
	#[default]
	Testnet,
	Development,
}

/// Helper macro to map derived metadata
#[macro_export]
macro_rules! metadata_scope {
	( $variant:expr, $block:block ) => {
		match $variant {
			$crate::metadata::Variant::Mainnet => {
				use $crate::metadata::timechain as metadata;
				$block
			},
			$crate::metadata::Variant::Staging => {
				use $crate::metadata::staging as metadata;
				$block
			},
			$crate::metadata::Variant::Testnet => {
				use $crate::metadata::testnet as metadata;
				$block
			},
			$crate::metadata::Variant::Development => {
				use $crate::metadata::development as metadata;
				$block
			},
		}
	};
}

/// Helper macro to map derived mainnet metadata
#[macro_export]
macro_rules! mainnet_scope {
	( $variant:expr, $block:block ) => {
		match $variant {
			$crate::metadata::Variant::Mainnet => {
				use $crate::metadata::timechain as metadata;
				$block
			},
			$crate::metadata::Variant::Staging => {
				use $crate::metadata::staging as metadata;
				$block
			},
			_ => {
				unimplemented!("mainnet only")
			},
		}
	};
}

/// Helper macro to map derived testnet metadata
#[macro_export]
macro_rules! testnet_scope {
	( $variant:expr, $block:block ) => {
		match $variant {
			$crate::metadata::Variant::Testnet => {
				use $crate::metadata::testnet as metadata;
				$block
			},
			$crate::metadata::Variant::Development => {
				use $crate::metadata::development as metadata;
				$block
			},
			_ => {
				unimplemented!("testnet only")
			},
		}
	};
}

/// Shared helper data structure
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug, scale_info::TypeInfo)]
pub enum MultiSigner {
	Ed25519([u8; 32]),
	Sr25519([u8; 32]),
	Ecdsa([u8; 33]),
}
