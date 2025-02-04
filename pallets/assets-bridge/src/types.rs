pub use polkadot_sdk::frame_support::traits::ExistenceRequirement;
use polkadot_sdk::frame_support::{dispatch::DispatchResult, pallet_prelude::RuntimeDebug};
use scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// NetworkDetails holds the current config of the network.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct NetworkDetails<Balance, NetworkData> {
	/// If this network is currently accepting teleports.
	pub active: bool,
	/// How much the base cost to teleport assets to this network.
	pub teleport_base_fee: Balance,
	/// Total amount of assets locked in this network.
	pub total_locked: Balance,
	/// Custom network data.
	pub data: NetworkData,
}
