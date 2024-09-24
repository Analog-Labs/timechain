use crate::{self as pallet_networks};
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use polkadot_sdk::{frame_support, frame_system};
use time_primitives::{ChainName, ChainNetwork};

#[test]
fn test_register_network() {
	let blockchain: ChainName = "Ethereum".into();
	let network: ChainNetwork = "Mainnet".into();
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::register_network(
			RawOrigin::Root.into(),
			42,
			blockchain.clone(),
			network.clone(),
			[0; 32],
			99,
		));
		assert_eq!(pallet_networks::Networks::<Test>::get(42), Some(42));
		assert_eq!(pallet_networks::NetworkName::<Test>::get(42), Some((blockchain, network)));
		assert_eq!(pallet_networks::NetworkGatewayAddress::<Test>::get(42), Some([0; 32]));
		assert_eq!(pallet_networks::NetworkGatewayBlock::<Test>::get(42), Some(99));
	});
}

#[test]
fn test_duplicate_insertion() {
	new_test_ext().execute_with(|| {
		assert_ok!(Networks::register_network(
			RawOrigin::Root.into(),
			42,
			"A".into(),
			"B".into(),
			[0; 32],
			0
		));
		assert_noop!(
			Networks::register_network(
				RawOrigin::Root.into(),
				42,
				"C".into(),
				"D".into(),
				[0; 32],
				0
			),
			<Error<Test>>::NetworkExists
		);
	});
}
