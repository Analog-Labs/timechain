use polkadot_sdk::frame_support::pallet_prelude::RuntimeDebug;
use scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// NetworkDetails holds the current config of the network.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct NetworkDetails<Balance, AccountId> {
	/// If this network is currently accepting teleports.
	pub active: bool,
	/// How much the base cost to teleport assets to this network.
	pub teleport_base_fee: Balance,
	/// Total amount of assets locked in this network.
	pub total_locked: Balance,
	/// Custom network data.
	pub data: NetworkData<AccountId>,
}

/// Network Data, akin to pallet_balances::AccountData
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct NetworkData<AccountId> {
	/// Nonce for the GMP message
	pub nonce: u64,
	/// Destination address for the token contract
	pub dest: AccountId,
}

impl<AccountId: Clone> NetworkData<AccountId> {
	pub fn dest(&self) -> AccountId {
		self.dest.clone()
	}

	pub fn incr_nonce(&mut self) -> Option<u64> {
		self.nonce.checked_add(1).and_then(|n| {
			self.nonce = n;
			Some(n)
		})
	}
}
