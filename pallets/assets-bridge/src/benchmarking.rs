use super::*;
use crate::Pallet;

use polkadot_sdk::*;

use frame_benchmarking::v2::*;
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use time_primitives::{traits::IdentifyAccount, AccountId, NetworkId, PublicKey, ANLOG};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const ETHEREUM: NetworkId = 1;

fn public_key() -> PublicKey {
	pk_from_account(ALICE)
}

fn pk_from_account(r: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(r))
}

#[benchmarks(
	where
		<T as pallet::Config>::NetworkId: From<u16>,
		<T as pallet::Config>::Beneficiary: From<[u8; 32]>,
		<<T as pallet::Config>::Currency as Currency<<T as polkadot_sdk::frame_system::Config>::AccountId>>::Balance: From<u128>,
)]
mod benchmarks {
	use super::*;
	use frame_support::traits::{Currency, ReservableCurrency};

	#[benchmark]
	fn teleport_keep_alive() {
		let caller = whitelisted_caller();
		let amount: BalanceOf<T> = (10_000 * ANLOG).into();
		T::Currency::resolve_creating(&caller, T::Currency::issue(amount));
		#[extrinsic_call]
		_(RawOrigin::Signed(caller), ETHEREUM.into(), BOB.into(), amount);
	}
}
