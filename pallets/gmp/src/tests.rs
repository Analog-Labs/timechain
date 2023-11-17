use crate::mock::*;
use crate::{Event, GatewayAddress, ShardRegistry};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::Network;

#[test]
fn deploy_gateway_works() {
	let contract_address = [0u8; 20];
	new_test_ext().execute_with(|| {
		assert!(GatewayAddress::<Test>::get(Network::Ethereum).is_none());
		assert!(ShardRegistry::<Test>::get(1, Network::Ethereum).is_none());
		assert!(ShardRegistry::<Test>::get(2, Network::Ethereum).is_none());
		assert!(ShardRegistry::<Test>::get(3, Network::Ethereum).is_none());
		assert_ok!(Gmp::deploy_gateway(
			RawOrigin::Root.into(),
			1u64,
			contract_address.clone(),
			sp_std::vec![1, 2, 3]
		));
		System::assert_last_event(
			Event::<Test>::GatewayContractDeployed(Network::Ethereum, contract_address.clone())
				.into(),
		);
		assert_eq!(GatewayAddress::<Test>::get(Network::Ethereum).unwrap(), contract_address);
		assert!(ShardRegistry::<Test>::get(1, Network::Ethereum).is_some());
		assert!(ShardRegistry::<Test>::get(2, Network::Ethereum).is_some());
		assert!(ShardRegistry::<Test>::get(3, Network::Ethereum).is_some());
	});
}

#[test]
fn register_shards_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Gmp::deploy_gateway(RawOrigin::Root.into(), 1u64, [0u8; 20], sp_std::vec![1]));
		assert!(ShardRegistry::<Test>::get(1, Network::Ethereum).is_some());
		assert!(ShardRegistry::<Test>::get(2, Network::Ethereum).is_none());
		assert!(ShardRegistry::<Test>::get(3, Network::Ethereum).is_none());
		assert_ok!(Gmp::register_shards(RawOrigin::Root.into(), 1u64, sp_std::vec![2, 3]));
		System::assert_last_event(Event::<Test>::ShardsRegistered(Network::Ethereum).into());
		assert!(ShardRegistry::<Test>::get(1, Network::Ethereum).is_some());
		assert!(ShardRegistry::<Test>::get(2, Network::Ethereum).is_some());
		assert!(ShardRegistry::<Test>::get(3, Network::Ethereum).is_some());
	});
}

#[test]
fn revoke_shards_works_works() {
	let contract_address = [0u8; 20];
	new_test_ext().execute_with(|| {
		assert_ok!(Gmp::deploy_gateway(
			RawOrigin::Root.into(),
			1u64,
			contract_address.clone(),
			sp_std::vec![1, 2, 3]
		));
		assert!(ShardRegistry::<Test>::get(1, Network::Ethereum).is_some());
		assert!(ShardRegistry::<Test>::get(2, Network::Ethereum).is_some());
		assert!(ShardRegistry::<Test>::get(3, Network::Ethereum).is_some());
		assert_ok!(Gmp::revoke_shards(RawOrigin::Root.into(), 1u64, sp_std::vec![2, 3]));
		System::assert_last_event(Event::<Test>::ShardsRevoked(Network::Ethereum).into());
		assert_eq!(GatewayAddress::<Test>::get(Network::Ethereum).unwrap(), contract_address);
		assert!(ShardRegistry::<Test>::get(1, Network::Ethereum).is_some());
		assert!(ShardRegistry::<Test>::get(2, Network::Ethereum).is_none());
		assert!(ShardRegistry::<Test>::get(3, Network::Ethereum).is_none());
	});
}

// #[test]
// fn revoke_shards_works() {
// 	assert!(true);
// }

// #[test]
// fn execute_works() {
// 	assert!(true);
// }
