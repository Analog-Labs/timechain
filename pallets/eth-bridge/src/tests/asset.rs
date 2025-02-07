// This file is part of the SORA network and Polkaswap app.

// Copyright (c) 2020, 2021, Polka Biome Ltd. All rights reserved.
// SPDX-License-Identifier: BSD-4-Clause

// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:

// Redistributions of source code must retain the above copyright notice, this list
// of conditions and the following disclaimer.
// Redistributions in binary form must reproduce the above copyright notice, this
// list of conditions and the following disclaimer in the documentation and/or other
// materials provided with the distribution.
//
// All advertising materials mentioning features or use of this software must display
// the following acknowledgement: This product includes software developed by Polka Biome
// Ltd., SORA, and Polkaswap.
//
// Neither the name of the Polka Biome Ltd. nor the names of its contributors may be used
// to endorse or promote products derived from this software without specific prior written permission.

// THIS SOFTWARE IS PROVIDED BY Polka Biome Ltd. AS IS AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL Polka Biome Ltd. BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
// OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
// STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use super::mock::*;
use super::{Assets, Error, EthBridge};
use crate::common::AssetId;
use crate::requests::{AssetKind, IncomingRequest, IncomingTransactionRequestKind};
use crate::tests::mock::{get_account_id_from_seed, ExtBuilder};
use crate::tests::{
	approve_last_request, assert_incoming_request_done, request_incoming, ETH_NETWORK_ID,
};
use crate::{EthAddress, RegisteredSidechainToken};
use frame_support::sp_runtime::app_crypto::sp_core::{self, sr25519};
use frame_support::{assert_err, assert_ok};
use hex_literal::hex;
use polkadot_sdk::*;
use sp_core::H256;
use std::str::FromStr;

#[test]
fn should_not_mint_and_burn_sidechain_asset() {
	let (mut ext, state) = ExtBuilder::default().build();

	#[track_caller]
	fn check_invariant(asset_id: &AssetId, val: u32) {
		assert_eq!(Assets::total_issuance(asset_id).unwrap(), val.into());
	}

	ext.execute_with(|| {
		let net_id = ETH_NETWORK_ID;
		let alice = get_account_id_from_seed::<sr25519::Public>("Alice");
		let token_address = EthAddress::from(hex!("40fd72257597aa14c7231a7b1aaa29fce868f677"));
		let (asset_id, asset_kind) =
			EthBridge::get_asset_by_raw_asset_id(H256::zero(), &token_address, net_id)
				.unwrap()
				.unwrap();
		assert_eq!(asset_kind, AssetKind::Reservable);
		check_invariant(&asset_id, 350007);
		let tx_hash = request_incoming(
			alice.clone(),
			H256::from_slice(&[1u8; 32]),
			IncomingTransactionRequestKind::Transfer.into(),
			net_id,
		)
		.unwrap();
		let incoming_transfer = IncomingRequest::Transfer(crate::IncomingTransfer {
			from: EthAddress::from([1; 20]),
			to: alice.clone(),
			asset_id,
			asset_kind,
			amount: 100u32.into(),
			author: alice.clone(),
			tx_hash,
			at_height: 1,
			timepoint: Default::default(),
			network_id: ETH_NETWORK_ID,
			should_take_fee: false,
		});
		assert_incoming_request_done(&state, incoming_transfer.clone()).unwrap();
		check_invariant(&asset_id, 350007);
		assert_ok!(EthBridge::transfer_to_sidechain(
			RuntimeOrigin::signed(alice.clone()),
			asset_id,
			EthAddress::from_str("19E7E376E7C213B7E7e7e46cc70A5dD086DAff2A").unwrap(),
			100_u32.into(),
			net_id,
		));
		approve_last_request(&state, net_id).expect("request wasn't approved");
		check_invariant(&asset_id, 350007);
	});
}

#[test]
fn should_not_burn_or_mint_sidechain_owned_asset() {
	let (mut ext, state) = ExtBuilder::default().build();

	#[track_caller]
	fn check_invariant() {
		let total_existential_deposit = 7;
		assert_eq!(
			Assets::total_issuance(&AssetId::Balances).unwrap(),
			350000 + total_existential_deposit
		);
	}

	ext.execute_with(|| {
		let net_id = ETH_NETWORK_ID;
		let alice = get_account_id_from_seed::<sr25519::Public>("Alice");
		assert_eq!(
			EthBridge::registered_asset(net_id, AssetId::Balances).unwrap(),
			AssetKind::Reservable
		);
		check_invariant();
		let tx_hash = request_incoming(
			alice.clone(),
			H256::from_slice(&[1u8; 32]),
			IncomingTransactionRequestKind::Transfer.into(),
			net_id,
		)
		.unwrap();
		let incoming_transfer = IncomingRequest::Transfer(crate::IncomingTransfer {
			from: EthAddress::from([1; 20]),
			to: alice.clone(),
			asset_id: AssetId::Balances,
			asset_kind: AssetKind::Reservable,
			amount: 100u32.into(),
			author: alice.clone(),
			tx_hash,
			at_height: 1,
			timepoint: Default::default(),
			network_id: ETH_NETWORK_ID,
			should_take_fee: false,
		});
		assert_incoming_request_done(&state, incoming_transfer.clone()).unwrap();
		check_invariant();
		assert_ok!(EthBridge::transfer_to_sidechain(
			RuntimeOrigin::signed(alice.clone()),
			AssetId::Balances,
			EthAddress::from_str("19E7E376E7C213B7E7e7e46cc70A5dD086DAff2A").unwrap(),
			100_u32.into(),
			net_id,
		));
		approve_last_request(&state, net_id).expect("request wasn't approved");
		check_invariant();
	});
}

#[test]
fn should_register_and_find_asset_ids() {
	let (mut ext, _state) = ExtBuilder::default().build();
	ext.execute_with(|| {
		let net_id = ETH_NETWORK_ID;
		// gets a known asset
		let (asset_id, asset_kind) = EthBridge::get_asset_by_raw_asset_id(
			H256::zero(),
			&sp_core::H160(hex!("40fd72257597aa14c7231a7b1aaa29fce868f677")),
			net_id,
		)
		.unwrap()
		.unwrap();
		assert_eq!(asset_id, AssetId::Balances);
		assert_eq!(asset_kind, AssetKind::Reservable);
	});
}

#[test]
fn should_reserve_owned_asset_on_different_networks() {
	let mut builder = ExtBuilder::default();
	let net_id_0 = ETH_NETWORK_ID;
	let net_id_1 = builder.add_network(vec![], None, None, Default::default());
	let (mut ext, state) = builder.build();

	ext.execute_with(|| {
		let alice = get_account_id_from_seed::<sr25519::Public>("Alice");
		let asset_id = AssetId::Balances;
		let token_address_1 = EthAddress::from(hex!("e88f8313e61a97cec1871ee37fbbe2a8bf3ed1e4"));
		EthBridge::register_existing_sidechain_asset(
			RuntimeOrigin::root(),
			asset_id,
			token_address_1,
			net_id_1,
		)
		.unwrap();
		Assets::mint_to(&asset_id, &alice, &alice, 100u32.into()).unwrap();
		Assets::mint_to(
			&asset_id,
			&alice,
			&state.networks[&net_id_0].config.bridge_account_id,
			100u32.into(),
		)
		.unwrap();
		Assets::mint_to(
			&asset_id,
			&alice,
			&state.networks[&net_id_1].config.bridge_account_id,
			100u32.into(),
		)
		.unwrap();
		let supply = Assets::total_issuance(&asset_id).unwrap();
		assert_ok!(EthBridge::transfer_to_sidechain(
			RuntimeOrigin::signed(alice.clone()),
			asset_id,
			EthAddress::from_str("19E7E376E7C213B7E7e7e46cc70A5dD086DAff2A").unwrap(),
			50_u32.into(),
			net_id_0,
		));
		approve_last_request(&state, net_id_0).expect("request wasn't approved");
		assert_ok!(EthBridge::transfer_to_sidechain(
			RuntimeOrigin::signed(alice.clone()),
			asset_id,
			EthAddress::from_str("19E7E376E7C213B7E7e7e46cc70A5dD086DAff2A").unwrap(),
			50_u32.into(),
			net_id_1,
		));
		approve_last_request(&state, net_id_1).expect("request wasn't approved");
		assert_eq!(Assets::total_issuance(&asset_id).unwrap(), supply);

		let tx_hash = request_incoming(
			alice.clone(),
			H256::from_slice(&[1u8; 32]),
			IncomingTransactionRequestKind::Transfer.into(),
			net_id_0,
		)
		.unwrap();
		let incoming_transfer = IncomingRequest::Transfer(crate::IncomingTransfer {
			from: EthAddress::from([1; 20]),
			to: alice.clone(),
			asset_id,
			asset_kind: AssetKind::Reservable,
			amount: 50u32.into(),
			author: alice.clone(),
			tx_hash,
			at_height: 1,
			timepoint: Default::default(),
			network_id: net_id_0,
			should_take_fee: false,
		});
		assert_incoming_request_done(&state, incoming_transfer.clone()).unwrap();
		let tx_hash = request_incoming(
			alice.clone(),
			H256::from_slice(&[2; 32]),
			IncomingTransactionRequestKind::Transfer.into(),
			net_id_1,
		)
		.unwrap();
		let incoming_transfer = IncomingRequest::Transfer(crate::IncomingTransfer {
			from: EthAddress::from([1; 20]),
			to: alice.clone(),
			asset_id,
			asset_kind: AssetKind::Reservable,
			amount: 50u32.into(),
			author: alice.clone(),
			tx_hash,
			at_height: 1,
			timepoint: Default::default(),
			network_id: net_id_1,
			should_take_fee: false,
		});
		assert_incoming_request_done(&state, incoming_transfer.clone()).unwrap();
		assert_eq!(Assets::total_issuance(&asset_id).unwrap(), supply);
	});
}

#[test]
fn should_handle_sidechain_and_thischain_asset_on_different_networks() {
	let mut builder = ExtBuilder::default();
	let net_id_0 = ETH_NETWORK_ID;
	let net_id_1 = builder.add_network(vec![], None, None, Default::default());
	let (mut ext, state) = builder.build();

	ext.execute_with(|| {
		let alice = get_account_id_from_seed::<sr25519::Public>("Alice");
		// Register token on the first network.
		let token_address_0 = EthAddress::from(hex!("40fd72257597aa14c7231a7b1aaa29fce868f677"));
		let token_address_1 = EthAddress::from(hex!("e88f8313e61a97cec1871ee37fbbe2a8bf3ed1e4"));
		EthBridge::register_existing_sidechain_asset(
			RuntimeOrigin::root(),
			AssetId::Balances,
			token_address_1,
			net_id_1,
		)
		.unwrap();
		let asset_id = EthBridge::registered_sidechain_asset(net_id_0, &token_address_0)
			.expect("Asset wasn't found.");
		assert_eq!(EthBridge::registered_asset(net_id_0, asset_id).unwrap(), AssetKind::Reservable);
		assert_eq!(EthBridge::registered_asset(net_id_1, asset_id).unwrap(), AssetKind::Reservable);
		Assets::mint_to(
			&asset_id,
			&state.networks[&net_id_0].config.bridge_account_id,
			&state.networks[&net_id_1].config.bridge_account_id,
			100u32.into(),
		)
		.unwrap();
		let supply = Assets::total_issuance(&asset_id).unwrap();
		let tx_hash = request_incoming(
			alice.clone(),
			H256::from_slice(&[1u8; 32]),
			IncomingTransactionRequestKind::Transfer.into(),
			net_id_0,
		)
		.unwrap();
		let incoming_transfer = IncomingRequest::Transfer(crate::IncomingTransfer {
			from: EthAddress::from([1; 20]),
			to: alice.clone(),
			asset_id,
			asset_kind: AssetKind::Reservable,
			amount: 50u32.into(),
			author: alice.clone(),
			tx_hash,
			at_height: 1,
			timepoint: Default::default(),
			network_id: net_id_0,
			should_take_fee: false,
		});
		assert_incoming_request_done(&state, incoming_transfer.clone()).unwrap();

		assert_ok!(EthBridge::transfer_to_sidechain(
			RuntimeOrigin::signed(alice.clone()),
			asset_id,
			EthAddress::from_str("19E7E376E7C213B7E7e7e46cc70A5dD086DAff2A").unwrap(),
			50_u32.into(),
			net_id_1,
		));
		approve_last_request(&state, net_id_1).expect("request wasn't approved");

		let tx_hash = request_incoming(
			alice.clone(),
			H256::from_slice(&[2; 32]),
			IncomingTransactionRequestKind::Transfer.into(),
			net_id_1,
		)
		.unwrap();
		let incoming_transfer = IncomingRequest::Transfer(crate::IncomingTransfer {
			from: EthAddress::from([1; 20]),
			to: alice.clone(),
			asset_id,
			asset_kind: AssetKind::Reservable,
			amount: 50u32.into(),
			author: alice.clone(),
			tx_hash,
			at_height: 1,
			timepoint: Default::default(),
			network_id: net_id_1,
			should_take_fee: false,
		});
		assert_incoming_request_done(&state, incoming_transfer.clone()).unwrap();

		assert_ok!(EthBridge::transfer_to_sidechain(
			RuntimeOrigin::signed(alice.clone()),
			asset_id,
			EthAddress::from_str("19E7E376E7C213B7E7e7e46cc70A5dD086DAff2A").unwrap(),
			50_u32.into(),
			net_id_0,
		));
		approve_last_request(&state, net_id_0).expect("request wasn't approved");
		assert_eq!(Assets::total_issuance(&asset_id).unwrap(), supply);
	});
}

#[test]
fn should_remove_asset() {
	let (mut ext, _state) = ExtBuilder::default().build();

	ext.execute_with(|| {
		let net_id = ETH_NETWORK_ID;
		assert_ok!(EthBridge::remove_sidechain_asset(
			RuntimeOrigin::root(),
			AssetId::Balances,
			net_id,
		));
		assert!(EthBridge::registered_asset(net_id, AssetId::Balances).is_none());
	});
}

#[test]
fn should_register_removed_asset() {
	let (mut ext, _state) = ExtBuilder::default().build();

	ext.execute_with(|| {
		let net_id = ETH_NETWORK_ID;
		let token_address =
			RegisteredSidechainToken::<Runtime>::get(net_id, AssetId::Balances).unwrap();
		assert_ok!(EthBridge::remove_sidechain_asset(
			RuntimeOrigin::root(),
			AssetId::Balances,
			net_id,
		));
		assert!(EthBridge::registered_asset(net_id, AssetId::Balances).is_none());
		assert_ok!(EthBridge::register_existing_sidechain_asset(
			RuntimeOrigin::root(),
			AssetId::Balances,
			token_address,
			net_id,
		));
		assert!(EthBridge::registered_asset(net_id, AssetId::Balances).is_some());
	});
}

#[test]
fn should_not_register_existing_asset() {
	let (mut ext, _state) = ExtBuilder::default().build();

	ext.execute_with(|| {
		let net_id = ETH_NETWORK_ID;
		let token_address =
			RegisteredSidechainToken::<Runtime>::get(net_id, AssetId::Balances).unwrap();
		assert_err!(
			EthBridge::register_existing_sidechain_asset(
				RuntimeOrigin::root(),
				AssetId::Balances,
				token_address,
				net_id,
			),
			Error::TokenIsAlreadyAdded
		);
	});
}
