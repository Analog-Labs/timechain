use super::*;
use crate::Pallet;

use frame_system::RawOrigin;
use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::frame_system;
use scale_info::prelude::string::String;
use time_primitives::{Network, NetworkConfig};

//TODO: choose & enforce MAX in code
const MAX_LENGTH: u32 = 1000;

fn mock_network_config() -> NetworkConfig {
	NetworkConfig {
		batch_size: 32,
		batch_offset: 0,
		batch_gas_limit: 10_000,
		shard_task_limit: 10,
	}
}

fn mock_network(chain_name: String, chain_network: String) -> Network {
	Network {
		id: 42,
		chain_name,
		chain_network,
		gateway: [0; 32],
		gateway_block: 99,
		config: mock_network_config(),
	}
}

benchmarks! {
	register_network {
		let a in 1..MAX_LENGTH;
		let b in 1..MAX_LENGTH;
		let mut name = String::new();
		let mut network = String::new();
		for _ in 0..a {
			name.push('a');
		}
		for _ in 0..b {
			network.push('b');
		}
	}: _(RawOrigin::Root, mock_network(name, network))
	verify {}

	set_network_config {
		Pallet::<T>::register_network(RawOrigin::Root.into(), mock_network("Ethereum".into(), "Mainnet".into())).unwrap();
	}: _(RawOrigin::Root, 42, mock_network_config())

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
