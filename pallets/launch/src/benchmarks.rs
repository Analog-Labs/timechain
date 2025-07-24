#![cfg(feature = "runtime-benchmarks")]
use crate::{BalanceOf, BridgedChain, Call, Config, CurrencyOf, Pallet};

//use super::mock_helpers::*;
use polkadot_sdk::*;

use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

use time_primitives::{Balance, TARGET_ISSUANCE};

#[benchmarks(
	where
		BalanceOf<T>: From<Balance>,
)]
mod benchmarks {
	use super::*;

	use polkadot_sdk::frame_support::traits::Currency;

	#[benchmark]
	fn bridge() {
		let bridge_account = BridgedChain::Base.account_id::<T>();
		let bridge_issuance = BalanceOf::<T>::from(TARGET_ISSUANCE);
		let _ = CurrencyOf::<T>::deposit_creating(&bridge_account, bridge_issuance);

		#[extrinsic_call]
		_(RawOrigin::Signed(bridge_account), BridgedChain::Ethereum, [0u8; 20], bridge_issuance);
	}
}
