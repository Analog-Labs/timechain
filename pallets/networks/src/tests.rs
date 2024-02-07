use crate::{self as pallet_networks};
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use time_primitives::{ChainName, ChainNetwork};

#[test]
fn test_add_network() {
	let blockchain: ChainName = "Ethereum".into();
	let network: ChainNetwork = "Mainnet".into();
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::add_network(
			RawOrigin::Root.into(),
			blockchain.clone(),
			network.clone(),
		));
		assert_eq!(pallet_networks::Networks::<Test>::get(0), Some((blockchain, network)));
	});
}

#[test]
fn test_duplicate_insertion() {
	let blockchain: ChainName = "Ethereum".into();
	let network: ChainNetwork = "Mainnet".into();
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::add_network(
			RawOrigin::Root.into(),
			blockchain.clone(),
			network.clone(),
		));
		assert_noop!(
			Networks::add_network(RawOrigin::Root.into(), blockchain, network),
			<Error<Test>>::NetworkExists
		);
	});
}

#[test]
fn test_single_blockchain_multiple_networks() {
	let blockchain: ChainName = "Ethereum".into();
	let network: ChainNetwork = "Mainnet".into();
	let network2: ChainNetwork = "Testnet".into();
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::add_network(
			RawOrigin::Root.into(),
			blockchain.clone(),
			network.clone(),
		));
		assert_ok!(Networks::add_network(
			RawOrigin::Root.into(),
			blockchain.clone(),
			network2.clone(),
		));
		assert_eq!(pallet_networks::Networks::<Test>::get(0), Some((blockchain.clone(), network)));
		assert_eq!(pallet_networks::Networks::<Test>::get(1), Some((blockchain, network2)));
	});
}
