use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use sp_std::vec;
use time_primitives::{Network, PublicKey};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const CHARLIE: [u8; 32] = [3u8; 32];

benchmarks! {
	register_shard {
		let collector = PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(ALICE));
	}: _(RawOrigin::Root, Network::Ethereum, vec![ALICE, BOB, CHARLIE], collector, 1)
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
