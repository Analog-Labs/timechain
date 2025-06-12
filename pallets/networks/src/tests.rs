use crate::{self as pallet_networks};
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use polkadot_sdk::{frame_support, frame_system, sp_runtime};
use scale_codec::Encode;
use sp_runtime::BoundedVec;
use time_primitives::{BatchGasParams, ChainName, Network, NetworkConfig};

fn mock_network_config() -> NetworkConfig {
	NetworkConfig {
		batch_size: 32,
		batch_offset: 0,
		shard_task_limit: 10,
		shard_size: 3,
		shard_threshold: 2,
		batch_gas_params: BatchGasParams {
			batch_gas_limit: 500_000,
			batch_exec_gas: 10_000,
			reg_op_exec_gas: 20_000,
			unreg_op_exec_gas: 20_000,
			msg_op_exec_gas: 100_000,
			msg_byte_gas: 20,
		},
		max_gas_price: 100,
	}
}

fn mock_network() -> Network {
	Network {
		id: 42,
		chain_name: ChainName(BoundedVec::truncate_from("Ethereum".encode())),
		gateway: [0; 32],
		gateway_block: 99,
		config: mock_network_config(),
	}
}

#[test]
fn test_register_network() {
	let network = mock_network();
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::register_network(RawOrigin::Root.into(), network.clone(),));
		assert_eq!(pallet_networks::Networks::<Test>::get(42), Some(network.id));
		assert_eq!(pallet_networks::NetworkName::<Test>::get(42), Some(network.chain_name));
		assert_eq!(pallet_networks::NetworkGatewayAddress::<Test>::get(42), Some(network.gateway));
		assert_eq!(
			pallet_networks::NetworkGatewayBlock::<Test>::get(42),
			Some(network.gateway_block)
		);
		assert_eq!(pallet_networks::NetworkMaxGasPrice::<Test>::get(42), Some(100));
	});
}

#[test]
fn test_duplicate_insertion() {
	let network = mock_network();
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::register_network(RawOrigin::Root.into(), network.clone(),));
		assert_noop!(
			Networks::register_network(RawOrigin::Root.into(), network,),
			<Error<Test>>::NetworkExists
		);
	});
}
