use super::*;
use crate::Pallet;

use polkadot_sdk::*;

use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use time_primitives::{NetworkId, ANLOG};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const ETHEREUM: NetworkId = 1;

#[benchmarks(
	where
		<T as pallet::Config>::NetworkId: From<u16>,
	<T as pallet::Config>::Beneficiary: From<[u8; 32]>,
	<T as polkadot_sdk::frame_system::Config>::AccountId: From<[u8; 32]>,
		<<T as pallet::Config>::Currency as Currency<<T as polkadot_sdk::frame_system::Config>::AccountId>>::Balance: From<u128>,
)]
mod benchmarks {
	use super::*;
	use frame_support::traits::Currency;

	#[benchmark]
	fn teleport_keep_alive() {
		let nw_data = NetworkDataOf::<T> {
			nonce: 0,
			dest: [0u8; 32].into(),
		};

		let caller = whitelisted_caller();
		let amount_init: BalanceOf<T> = (1_000_000_000_000 * ANLOG).into();
		let amount_teleport: BalanceOf<T> = (10 * ANLOG).into();
		T::Currency::resolve_creating(&caller, T::Currency::issue(amount_init));

		let _ = Pallet::<T>::do_register_network(ETHEREUM.into(), ANLOG.into(), nw_data);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), ETHEREUM.into(), ALICE.into(), amount_teleport);
	}

	#[benchmark]
	fn force_teleport() {
		let nw_data = NetworkDataOf::<T> {
			nonce: 0,
			dest: [0u8; 32].into(),
		};
		let caller = whitelisted_caller();
		let amount_init: BalanceOf<T> = (1_000_000_000_000 * ANLOG).into();
		let amount_teleport: BalanceOf<T> = (10 * ANLOG).into();
		T::Currency::resolve_creating(&caller, T::Currency::issue(amount_init));

		let _ = Pallet::<T>::do_register_network(ETHEREUM.into(), ANLOG.into(), nw_data);

		#[extrinsic_call]
		_(RawOrigin::Root, caller, ETHEREUM.into(), ALICE.into(), amount_teleport);
	}

	#[benchmark]
	fn register_network() {
		let nw_data = NetworkDataOf::<T> {
			nonce: 0,
			dest: [0u8; 32].into(),
		};
		#[extrinsic_call]
		_(RawOrigin::Root, ETHEREUM.into(), ANLOG.into(), nw_data);
	}

	#[benchmark]
	fn force_update_network() {
		let nw_data = NetworkDataOf::<T> {
			nonce: 1,
			dest: [0u8; 32].into(),
		};
		let _ = Pallet::<T>::do_register_network(ETHEREUM.into(), ANLOG.into(), nw_data.clone());

		#[extrinsic_call]
		_(RawOrigin::Root, ETHEREUM.into(), false, Some(nw_data));
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
