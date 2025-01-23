use super::*;
use crate::Pallet;

use frame_system::RawOrigin;
use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::{frame_system, sp_runtime};
use scale_codec::Encode;
use scale_info::prelude::string::String;
use sp_runtime::BoundedVec;
use time_primitives::{
	ChainName, ChainNetwork, Network, NetworkConfig, NetworkId, CHAIN_NAME_LEN, CHAIN_NET_LEN,
};

const NETWORK: NetworkId = 42;

fn mock_network_config() -> NetworkConfig {
	NetworkConfig {
		batch_size: 32,
		batch_offset: 0,
		batch_gas_limit: 10_000,
		shard_task_limit: 10,
		shard_size: 3,
		shard_threshold: 2,
	}
}

fn mock_network(chain_name: String, chain_network: String) -> Network {
	Network {
		id: NETWORK,
		chain_name: ChainName(BoundedVec::truncate_from(chain_name.as_str().encode())),
		chain_network: ChainNetwork(BoundedVec::truncate_from(chain_network.as_str().encode())),
		gateway: [0; 32],
		gateway_block: 99,
		config: mock_network_config(),
	}
}

benchmarks! {
	register_network {
		let a in 1..CHAIN_NAME_LEN;
		let b in 1..CHAIN_NET_LEN;
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
	}: _(RawOrigin::Root, NETWORK, mock_network_config())
	verify {}

	remove_network {
		Pallet::<T>::register_network(RawOrigin::Root.into(), mock_network("Ethereum".into(), "Mainnet".into())).unwrap();
	}: _(RawOrigin::Root, NETWORK)
	verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
