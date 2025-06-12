use super::*;
use crate::Pallet;

use frame_system::RawOrigin;
use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::{frame_system, sp_runtime};
use scale_codec::Encode;
use scale_info::prelude::string::String;
use sp_runtime::BoundedVec;
use time_primitives::{ChainName, Network, NetworkConfig, NetworkId, CHAIN_NAME_LEN};

const NETWORK: NetworkId = 42;

fn mock_network_config() -> NetworkConfig {
	NetworkConfig {
		batch_size: 32,
		batch_offset: 0,
		shard_task_limit: 10,
		shard_size: 3,
		shard_threshold: 2,
		batch_gas_params: Default::default(),
		max_gas_price: 100,
	}
}

fn mock_network(chain_name: String) -> Network {
	Network {
		id: NETWORK,
		chain_name: ChainName(BoundedVec::truncate_from(chain_name.as_str().encode())),
		gateway: [0; 32],
		gateway_block: 99,
		config: mock_network_config(),
	}
}

benchmarks! {
	register_network {
		let a in 1..CHAIN_NAME_LEN;
		let mut name = String::new();
		for _ in 0..a {
			name.push('a');
		}
	}: _(RawOrigin::Root, mock_network(name))
	verify {}

	set_network_config {
		Pallet::<T>::register_network(RawOrigin::Root.into(), mock_network("Ethereum Mainnet".into())).unwrap();
	}: _(RawOrigin::Root, NETWORK, mock_network_config())
	verify {}

	remove_network {
		Pallet::<T>::register_network(RawOrigin::Root.into(), mock_network("Ethereum Mainnet".into())).unwrap();
	}: _(RawOrigin::Root, NETWORK)
	verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
