#[subxt::subxt(
	runtime_metadata_path = "../../config/subxt/mainnet.default.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain {}

#[subxt::subxt(
	runtime_metadata_path = "../../config/subxt/mainnet.development.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod staging {}

#[subxt::subxt(
	runtime_metadata_path = "../../config/subxt/testnet.default.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod testnet {}

#[subxt::subxt(
	runtime_metadata_path = "../../config/subxt/testnet.development.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod development {}
