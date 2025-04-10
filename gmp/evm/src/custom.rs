//! Custom chains logic
use alloy::{eips::eip1559::Eip1559Estimation, providers::utils::Eip1559EstimatorFn};

const BEP226_MIN_PRIORITY_FEE: u128 = 1000000000;

/// BNB EIP1559 estimator.
/// See [`BEP226`](https://github.com/bnb-chain/BEPs/pull/226).
pub(crate) struct BEP226;

impl Eip1559EstimatorFn for BEP226 {
	fn estimate(&self, _base_fee: u128, rewards: &[Vec<u128>]) -> Eip1559Estimation {
		let fee = rewards
			.iter()
			.flatten()
			.next()
			.map(|f| f.max(&BEP226_MIN_PRIORITY_FEE))
			.unwrap_or(&BEP226_MIN_PRIORITY_FEE);

		Eip1559Estimation {
			max_fee_per_gas: *fee,
			max_priority_fee_per_gas: *fee,
		}
	}
}
