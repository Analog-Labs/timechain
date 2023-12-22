use crate::{self as pallet_networks};
use crate::{mock::*, Error, NetworkIdToNetwork};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use frame_system::RawOrigin;
use time_primitives::{NetworkBlockchain, NetworkName};

fn blockchain_to_vec(
	data: String,
) -> BoundedVec<u8, <Test as pallet_networks::Config>::MaxNameSize> {
	BoundedVec::<u8, <Test as pallet_networks::Config>::MaxBlockchainSize>::try_from(
		data.trim().to_string().into_bytes(),
	)
	.unwrap()
}
fn network_to_vec(data: String) -> BoundedVec<u8, <Test as pallet_networks::Config>::MaxNameSize> {
	BoundedVec::<u8, <Test as pallet_networks::Config>::MaxNameSize>::try_from(
		data.trim().to_string().into_bytes(),
	)
	.unwrap()
}

#[test]
fn test_add_network() {
	let blockchain: NetworkBlockchain = "Ethereum".into();
	let network: NetworkName = "Mainnet".into();
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::add_network(
			RawOrigin::Root.into(),
			blockchain.clone(),
			network.clone()
		));
		assert_eq!(
			NetworkIdToNetwork::<Test>::get(0),
			Some((blockchain_to_vec(blockchain), network_to_vec(network)))
		);
	});
}

#[test]
fn test_duplicate_insertion() {
	let blockchain: NetworkBlockchain = "Ethereum".into();
	let network: NetworkName = "Mainnet".into();
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::add_network(
			RawOrigin::Root.into(),
			blockchain.clone(),
			network.clone()
		));
		assert_noop!(
			Networks::add_network(RawOrigin::Root.into(), blockchain, network),
			<Error<Test>>::NetworkAlreadyExists
		);
	});
}

#[test]
fn test_single_blockchain_multiple_networks() {
	let blockchain: NetworkBlockchain = "Ethereum".into();
	let network: NetworkName = "Mainnet".into();
	let network2: NetworkName = "Testnet".into();
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::add_network(
			RawOrigin::Root.into(),
			blockchain.clone(),
			network.clone()
		));
		assert_ok!(Networks::add_network(
			RawOrigin::Root.into(),
			blockchain.clone(),
			network2.clone()
		));
		assert_eq!(
			NetworkIdToNetwork::<Test>::get(0),
			Some((blockchain_to_vec(blockchain.clone()), network_to_vec(network)))
		);
		assert_eq!(
			NetworkIdToNetwork::<Test>::get(1),
			Some((blockchain_to_vec(blockchain), network_to_vec(network2)))
		);
	});
}
