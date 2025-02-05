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
		<T as pallet::Config>::NetworkData: From<(u64, [u8; 32])>,
		<T as pallet::Config>::Beneficiary: From<[u8; 32]>,
		<<T as pallet::Config>::Currency as Currency<<T as polkadot_sdk::frame_system::Config>::AccountId>>::Balance: From<u128>,
)]
mod benchmarks {
	use super::*;
	use frame_support::traits::Currency;

	#[benchmark]
	fn teleport_keep_alive() {
		let caller = whitelisted_caller();
		let amount_init: BalanceOf<T> = (1_000_000_000_000 * ANLOG).into();
		let amount_teleport: BalanceOf<T> = (10 * ANLOG).into();
		T::Currency::resolve_creating(&caller, T::Currency::issue(amount_init));

		let _ = Pallet::<T>::do_register_network(
			ETHEREUM.into(),
			(1 * ANLOG).into(),
			(0, [0u8; 32]).into(),
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), ETHEREUM.into(), ALICE.into(), amount_teleport);
	}

	#[benchmark]
	fn force_teleport() {
		let caller = whitelisted_caller();
		let amount_init: BalanceOf<T> = (1_000_000_000_000 * ANLOG).into();
		let amount_teleport: BalanceOf<T> = (10 * ANLOG).into();
		T::Currency::resolve_creating(&caller, T::Currency::issue(amount_init));

		let _ = Pallet::<T>::do_register_network(
			ETHEREUM.into(),
			(1 * ANLOG).into(),
			(0, [0u8; 32]).into(),
		);

		#[extrinsic_call]
		_(RawOrigin::Root, caller.into(), ETHEREUM.into(), ALICE.into(), amount_teleport);
	}

	// #[benchmark]
	// fn register_network() {
	// 	let caller = whitelisted_caller();
	// 	let amount_init: BalanceOf<T> = (1_000_000_000_000 * ANLOG).into();
	// 	let amount_teleport: BalanceOf<T> = (1 * ANLOG).into();
	// 	T::Currency::resolve_creating(&caller, T::Currency::issue(amount_init));

	// 	let _ = Pallet::<T>::do_register_network(
	// 		ETHEREUM.into(),
	// 		// TODO add fee here
	// 		Default::default(),
	// 		(0, [0u8; 32]).into(),
	// 	);

	// 	#[extrinsic_call]
	// 	_(RawOrigin::Root, ETHEREUM.into(), ALICE.into(), amount_teleport);
	// }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
