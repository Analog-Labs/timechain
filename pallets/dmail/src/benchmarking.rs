use super::*;
use crate::Pallet;

use scale_info::prelude::string::String;

use frame_system::RawOrigin;
use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::frame_system;
use polkadot_sdk::sp_runtime::BoundedVec;
use scale_codec::Encode;
use time_primitives::{DMAIL_PATH_LEN, DMAIL_TO_LEN};

benchmarks! {
	send_email {
		let a in 1..DMAIL_TO_LEN;
		let b in 1..DMAIL_PATH_LEN;

		let mut to = String::new();
		let mut path = String::new();
		for _ in 0..a {
			to.push('a');
		}
		for _ in 0..b {
			path.push('b');
		}
		let to = BoundedVec::truncate_from(to.as_str().encode());
		let path = BoundedVec::truncate_from(path.as_str().encode());
	}: _(RawOrigin::Signed([0u8; 32].into()), to, path)
	verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
