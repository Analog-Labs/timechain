use super::*;
use crate::Pallet;

use frame_system::RawOrigin;
use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::{frame_system, sp_runtime, sp_std};
use sp_runtime::BoundedVec;
use sp_std::vec;
use time_primitives::{DmailPath, DmailTo, DMAIL_PATH_LEN, DMAIL_TO_LEN};

benchmarks! {
	send_email {
		let to = DmailTo(BoundedVec::truncate_from(vec![0u8; DMAIL_TO_LEN.try_into().unwrap_or_default()]));
		let path = DmailPath(BoundedVec::truncate_from(vec![ 0u8; DMAIL_PATH_LEN.try_into().unwrap_or_default()]));
	}: _(RawOrigin::Signed([0u8; 32].into()), to, path)
	verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
