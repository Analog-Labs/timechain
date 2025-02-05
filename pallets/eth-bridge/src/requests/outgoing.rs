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

use crate::common;
use crate::common::{AssetId, Assets, Balance};
use crate::offchain::SignatureParams;
use crate::types::EthAddress;
use crate::util::{get_bridge_account, Decoder};
use crate::{
	types, BridgeNetworkId, BridgeSignatureVersion, BridgeSignatureVersions, BridgeStatus,
	BridgeTimepoint, Config, Error, OffchainRequest, OutgoingRequest, Pallet, MAX_PEERS, MIN_PEERS,
};
use alloc::collections::BTreeSet;
use alloc::string::String;
use ethabi::Token;
#[allow(unused_imports)]
use frame_support::debug;
use frame_support::sp_runtime::app_crypto::sp_core;
use polkadot_sdk::*;
use scale_codec::{Decode, Encode};
use sp_runtime::DispatchError;

#[cfg(feature = "std")]
use crate::common::string_serialization;
use frame_support::ensure;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::RuntimeDebug;
use sp_core::{H256, U256};
use sp_std::convert::TryInto;
use sp_std::prelude::*;

/// Outgoing request for transferring the given asset from Thischain to Sidechain.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct OutgoingTransfer<T: Config> {
	pub from: T::AccountId,
	pub to: EthAddress,
	pub asset_id: AssetId,
	#[cfg_attr(feature = "std", serde(with = "string_serialization"))]
	pub amount: Balance,
	pub nonce: T::Nonce,
	pub network_id: BridgeNetworkId<T>,
	pub timepoint: BridgeTimepoint<T>,
}

impl<T: Config> OutgoingTransfer<T> {
	pub fn sidechain_amount(&self) -> Result<(u128, Balance), Error<T>> {
		Ok((self.amount, 0))
	}

	pub fn to_eth_abi(&self, tx_hash: H256) -> Result<OutgoingTransferEncoded, Error<T>> {
		// TODO: Incorrect type (Address != AccountId).
		let from = EthAddress::from_slice(&self.from.encode()[..20]);
		let to = self.to;
		let amount = U256::from(self.amount);
		let token_address =
			Pallet::<T>::registered_sidechain_token(self.network_id, &self.asset_id)
				.ok_or(Error::<T>::UnknownAssetId)?;
		let currency_id = CurrencyIdEncoded::TokenAddress(token_address);

		let tx_hash = H256(tx_hash.0);
		let mut network_id: H256 = H256::default();
		U256::from(
			<T::NetworkId as TryInto<u128>>::try_into(self.network_id)
				.ok()
				.expect("NetworkId can be always converted to u128; qed"),
		)
		.to_big_endian(&mut network_id.0);
		let signature_version = BridgeSignatureVersions::<T>::get(self.network_id);
		let raw = match signature_version {
			BridgeSignatureVersion::V3 => {
				let prefix = "transfer";
				ethabi::encode(&[
					Token::String(prefix.into()),
					Token::Address(
						crate::BridgeContractAddress::<T>::get(self.network_id).0.into(),
					),
					currency_id.to_token(),
					Token::Uint(amount),
					Token::Address(types::H160(to.0)),
					Token::Address(types::H160(from.0)),
					Token::FixedBytes(tx_hash.0.to_vec()),
					Token::FixedBytes(network_id.0.to_vec()),
				])
			},
		};
		Ok(OutgoingTransferEncoded {
			from,
			to,
			currency_id,
			amount,
			tx_hash,
			network_id,
			raw,
		})
	}

	/// Checks that the given asset can be transferred through the bridge.
	pub fn validate(&self) -> Result<(), DispatchError> {
		ensure!(
			crate::RegisteredAsset::<T>::get(self.network_id, &self.asset_id).is_some(),
			Error::<T>::UnsupportedToken
		);
		Ok(())
	}

	/// Transfers the given `amount` of `asset_id` to the bridge account and reserve it.
	pub fn prepare(&self, _tx_hash: H256) -> Result<(), DispatchError> {
		let bridge_account = get_bridge_account::<T>(self.network_id);
		common::with_transaction(|| {
			Assets::<T>::transfer_from(&self.asset_id, &self.from, &bridge_account, self.amount)?;
			Assets::<T>::reserve(&self.asset_id, &bridge_account, self.amount)?;
			Ok(())
		})
	}

	pub fn cancel(&self) -> Result<(), DispatchError> {
		let bridge_account = get_bridge_account::<T>(self.network_id);
		common::with_transaction(|| {
			Assets::<T>::unreserve(&self.asset_id, &bridge_account, self.amount)?;
			Assets::<T>::transfer_from(&self.asset_id, &bridge_account, &self.from, self.amount)?;
			Ok(())
		})
	}

	/// Validates the request again, then, if the asset is originated in Sidechain, it gets burned.
	pub fn finalize(&self, _tx_hash: H256) -> Result<(), DispatchError> {
		self.validate()?;
		let bridge_acc = get_bridge_account::<T>(self.network_id);
		crate::common::with_transaction(|| {
			Assets::<T>::unreserve(&self.asset_id, &bridge_acc, self.amount)?;
			Ok(())
		})
	}
}

/// Thischain or Sidechain asset id.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyIdEncoded {
	TokenAddress(EthAddress),
}

impl CurrencyIdEncoded {
	pub fn to_token(&self) -> Token {
		match self {
			CurrencyIdEncoded::TokenAddress(address) => Token::Address(types::H160(address.0)),
		}
	}
}

/// Sidechain-compatible version of `OutgoingTransfer`.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OutgoingTransferEncoded {
	pub currency_id: CurrencyIdEncoded,
	pub amount: U256,
	pub to: EthAddress,
	pub from: EthAddress,
	pub tx_hash: H256,
	pub network_id: H256,
	/// EABI-encoded data to be signed.
	pub raw: Vec<u8>,
}

impl OutgoingTransferEncoded {
	pub fn input_tokens(&self, signatures: Option<Vec<SignatureParams>>) -> Vec<Token> {
		let mut tokens = vec![
			self.currency_id.to_token(),
			Token::Uint(types::U256(self.amount.0)),
			Token::Address(types::H160(self.to.0)),
			Token::Address(types::H160(self.from.0)),
			Token::FixedBytes(self.tx_hash.0.to_vec()),
		];

		if let Some(sigs) = signatures {
			let sig_tokens = signature_params_to_tokens(sigs);
			tokens.extend(sig_tokens);
		}
		tokens
	}
}

#[derive(Default)]
pub struct Encoder {
	tokens: Vec<Token>,
}

impl Encoder {
	pub fn new() -> Self {
		Encoder::default()
	}

	pub fn write_address(&mut self, val: &EthAddress) {
		self.tokens.push(Token::Address(types::H160(val.0)));
	}

	pub fn write_string(&mut self, val: String) {
		self.tokens.push(Token::String(val));
	}

	pub fn write_u8(&mut self, val: u8) {
		self.tokens.push(Token::Uint(types::U256::from(val)));
	}

	pub fn into_inner(self) -> Vec<Token> {
		self.tokens
	}
}

/// Converts signature params to Sidechain-compatible tokens.
pub fn signature_params_to_tokens(sig_params: Vec<SignatureParams>) -> Vec<Token> {
	let mut vs = Vec::new();
	let mut rs = Vec::new();
	let mut ss = Vec::new();
	for sig_param in sig_params {
		vs.push(Token::Uint(types::U256::from(sig_param.v)));
		rs.push(Token::FixedBytes(sig_param.r.to_vec()));
		ss.push(Token::FixedBytes(sig_param.s.to_vec()));
	}
	vec![Token::Array(vs), Token::Array(rs), Token::Array(ss)]
}

/// Outgoing request for adding a peer.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct OutgoingAddPeer<T: Config> {
	pub author: T::AccountId,
	pub peer_address: EthAddress,
	pub peer_account_id: T::AccountId,
	pub nonce: T::Nonce,
	pub network_id: BridgeNetworkId<T>,
	pub timepoint: BridgeTimepoint<T>,
}

impl<T: Config> OutgoingAddPeer<T> {
	pub fn to_eth_abi(&self, tx_hash: H256) -> Result<OutgoingAddPeerEncoded, Error<T>> {
		let tx_hash = H256(tx_hash.0);
		let peer_address = self.peer_address;
		let mut network_id: H256 = H256::default();
		U256::from(
			<T::NetworkId as TryInto<u128>>::try_into(self.network_id)
				.ok()
				.expect("NetworkId can be always converted to u128; qed"),
		)
		.to_big_endian(&mut network_id.0);
		let signature_version = BridgeSignatureVersions::<T>::get(self.network_id);
		let raw = match signature_version {
			BridgeSignatureVersion::V3 => ethabi::encode(&[
				Token::String("addPeer".into()),
				Token::Address(crate::BridgeContractAddress::<T>::get(self.network_id).0.into()),
				Token::Address(types::H160(peer_address.0)),
				Token::FixedBytes(tx_hash.0.to_vec()),
				Token::FixedBytes(network_id.0.to_vec()),
			]),
		};
		Ok(OutgoingAddPeerEncoded {
			peer_address,
			tx_hash,
			network_id,
			raw,
		})
	}

	/// Checks that the current number of peers is not greater than `MAX_PEERS` and the given peer
	/// is not presented in the current peer set,
	pub fn validate(&self) -> Result<BTreeSet<T::AccountId>, DispatchError> {
		let peers = crate::Peers::<T>::get(self.network_id);
		ensure!(peers.len() <= MAX_PEERS, Error::<T>::CantAddMorePeers);
		ensure!(!peers.contains(&self.peer_account_id), Error::<T>::PeerIsAlreadyAdded);
		Ok(peers)
	}

	/// Checks that the current pending peer value is none and inserts the given one.
	pub fn prepare(&self, _validated_state: ()) -> Result<(), DispatchError> {
		let pending_peer = crate::PendingPeer::<T>::get(self.network_id);
		ensure!(pending_peer.is_none(), Error::<T>::TooManyPendingPeers);
		frame_system::Pallet::<T>::inc_consumers(&self.peer_account_id)
			.map_err(|_| Error::<T>::IncRefError)?;
		crate::PendingPeer::<T>::insert(self.network_id, self.peer_account_id.clone());
		Ok(())
	}

	/// Calls `validate` again and inserts the peer account ids on Thischain and Sidechain to
	/// have an association.
	pub fn finalize(&self) -> Result<(), DispatchError> {
		let _peers = self.validate()?;
		crate::PeerAccountId::<T>::insert(
			self.network_id,
			self.peer_address,
			self.peer_account_id.clone(),
		);
		crate::PeerAddress::<T>::insert(self.network_id, &self.peer_account_id, self.peer_address);
		Ok(())
	}

	/// Cleans the current pending peer value.
	pub fn cancel(&self) -> Result<(), DispatchError> {
		if let Some(account_id) = crate::PendingPeer::<T>::take(self.network_id) {
			frame_system::Pallet::<T>::dec_consumers(&account_id);
		}
		Ok(())
	}
}

/// Outgoing request for removing a peer.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct OutgoingRemovePeer<T: Config> {
	pub author: T::AccountId,
	pub peer_account_id: T::AccountId,
	pub peer_address: EthAddress,
	pub nonce: T::Nonce,
	pub network_id: BridgeNetworkId<T>,
	pub timepoint: BridgeTimepoint<T>,
}

impl<T: Config> OutgoingRemovePeer<T> {
	pub fn to_eth_abi(&self, tx_hash: H256) -> Result<OutgoingRemovePeerEncoded, Error<T>> {
		let tx_hash = H256(tx_hash.0);
		let peer_address = self.peer_address;
		let mut network_id: H256 = H256::default();
		U256::from(
			<T::NetworkId as TryInto<u128>>::try_into(self.network_id)
				.ok()
				.expect("NetworkId can be always converted to u128; qed"),
		)
		.to_big_endian(&mut network_id.0);
		let signature_version = BridgeSignatureVersions::<T>::get(self.network_id);
		let raw = match signature_version {
			BridgeSignatureVersion::V3 => ethabi::encode(&[
				Token::String("removePeer".into()),
				Token::Address(crate::BridgeContractAddress::<T>::get(self.network_id).0.into()),
				Token::Address(types::H160(peer_address.0)),
				Token::FixedBytes(tx_hash.0.to_vec()),
				Token::FixedBytes(network_id.0.to_vec()),
			]),
		};
		Ok(OutgoingRemovePeerEncoded {
			peer_address,
			tx_hash,
			network_id,
			raw,
		})
	}

	/// Checks that the current number of peers is not less than `MIN_PEERS` and the given peer
	/// is presented in the current peer set,
	pub fn validate(&self) -> Result<BTreeSet<T::AccountId>, DispatchError> {
		let peers = crate::Peers::<T>::get(self.network_id);
		ensure!(peers.len() >= MIN_PEERS, Error::<T>::CantRemoveMorePeers);
		ensure!(peers.contains(&self.peer_account_id), Error::<T>::UnknownPeerId);
		Ok(peers)
	}

	/// Checks that the current pending peer value is none and inserts the given one.
	pub fn prepare(&self, _validated_state: ()) -> Result<(), DispatchError> {
		let pending_peer = crate::PendingPeer::<T>::get(self.network_id);
		ensure!(pending_peer.is_none(), Error::<T>::TooManyPendingPeers);
		frame_system::Pallet::<T>::inc_consumers(&self.peer_account_id)
			.map_err(|_| Error::<T>::IncRefError)?;
		crate::PendingPeer::<T>::insert(self.network_id, self.peer_account_id.clone());
		Ok(())
	}

	/// Calls `validate` again and removes the peer from the peer set and from the multisig bridge
	/// account.
	pub fn finalize(&self) -> Result<(), DispatchError> {
		let mut peers = self.validate()?;
		bridge_multisig::Pallet::<T>::remove_signatory(
			get_bridge_account::<T>(self.network_id),
			self.peer_account_id.clone(),
		)?;
		peers.remove(&self.peer_account_id);
		crate::Peers::<T>::insert(self.network_id, peers);
		// TODO: check it's not conflicting with compat request
		crate::PeerAccountId::<T>::take(self.network_id, self.peer_address);
		crate::PeerAddress::<T>::take(self.network_id, &self.peer_account_id);
		Ok(())
	}

	/// Cleans the current pending peer value.
	pub fn cancel(&self) -> Result<(), DispatchError> {
		if let Some(account_id) = crate::PendingPeer::<T>::take(self.network_id) {
			frame_system::Pallet::<T>::dec_consumers(&account_id);
		}
		Ok(())
	}

	pub fn should_be_skipped(&self) -> bool {
		false
	}
}

/// Sidechain-compatible version of `OutgoingAddPeer`.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OutgoingAddPeerEncoded {
	pub peer_address: EthAddress,
	pub tx_hash: H256,
	pub network_id: H256,
	/// EABI-encoded data to be signed.
	pub raw: Vec<u8>,
}

impl OutgoingAddPeerEncoded {
	pub fn input_tokens(&self, signatures: Option<Vec<SignatureParams>>) -> Vec<Token> {
		let mut tokens = vec![
			Token::Address(types::H160(self.peer_address.0)),
			Token::FixedBytes(self.tx_hash.0.to_vec()),
		];
		if let Some(sigs) = signatures {
			let sig_tokens = signature_params_to_tokens(sigs);
			tokens.extend(sig_tokens);
		}
		tokens
	}
}

/// Sidechain-compatible version of `OutgoingRemovePeer`.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OutgoingRemovePeerEncoded {
	pub peer_address: EthAddress,
	pub tx_hash: H256,
	pub network_id: H256,
	/// EABI-encoded data to be signed.
	pub raw: Vec<u8>,
}

impl OutgoingRemovePeerEncoded {
	pub fn input_tokens(&self, signatures: Option<Vec<SignatureParams>>) -> Vec<Token> {
		let mut tokens = vec![
			Token::Address(types::H160(self.peer_address.0)),
			Token::FixedBytes(self.tx_hash.0.to_vec()),
		];
		if let Some(sigs) = signatures {
			let sig_tokens = signature_params_to_tokens(sigs);
			tokens.extend(sig_tokens);
		}
		tokens
	}
}

/// Outgoing request for preparing bridge for migration.
///
/// The migration is executed in 2 phases:
/// 1. Prepare both chains for migration. After the preparation, Thischain stops collecting
/// signatures for outgoing requests, but accepts all incoming requests. This phase is used to
/// get pending incoming requests to finish and to have both chains as much synchronised
/// as possible.
/// 2. Migrate the bridge. At this stage a new Sidechain contract should be deployed and Thischain
/// should be switched to it, so the old contract can't be used anymore.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct OutgoingPrepareForMigration<T: Config> {
	pub author: T::AccountId,
	pub nonce: T::Nonce,
	pub network_id: BridgeNetworkId<T>,
	pub timepoint: BridgeTimepoint<T>,
}

impl<T: Config> OutgoingPrepareForMigration<T> {
	pub fn to_eth_abi(
		&self,
		tx_hash: H256,
	) -> Result<OutgoingPrepareForMigrationEncoded, Error<T>> {
		let tx_hash = H256(tx_hash.0);
		let mut network_id: H256 = H256::default();
		U256::from(
			<T::NetworkId as TryInto<u128>>::try_into(self.network_id)
				.ok()
				.expect("NetworkId can be always converted to u128; qed"),
		)
		.to_big_endian(&mut network_id.0);
		let contract_address: EthAddress = crate::BridgeContractAddress::<T>::get(&self.network_id);
		let signature_version = BridgeSignatureVersions::<T>::get(self.network_id);
		let raw = match signature_version {
			BridgeSignatureVersion::V3 => ethabi::encode(&[
				Token::String("prepareMigration".into()),
				Token::Address(types::EthAddress::from(contract_address.0)),
				Token::FixedBytes(tx_hash.0.to_vec()),
				Token::FixedBytes(network_id.0.to_vec()),
			]),
		};
		Ok(OutgoingPrepareForMigrationEncoded {
			this_contract_address: contract_address,
			tx_hash,
			network_id,
			raw,
		})
	}

	pub fn validate(&self) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn prepare(&self, _validated_state: ()) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn cancel(&self) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn finalize(&self) -> Result<(), DispatchError> {
		Ok(())
	}
}

/// Sidechain-compatible version of `OutgoingPrepareForMigration`.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OutgoingPrepareForMigrationEncoded {
	pub this_contract_address: EthAddress,
	pub tx_hash: H256,
	pub network_id: H256,
	/// EABI-encoded data to be signed.
	pub raw: Vec<u8>,
}

impl OutgoingPrepareForMigrationEncoded {
	pub fn input_tokens(&self, signatures: Option<Vec<SignatureParams>>) -> Vec<Token> {
		let mut tokens = vec![
			Token::Address(types::EthAddress::from(self.this_contract_address.0)),
			Token::FixedBytes(self.tx_hash.0.to_vec()),
		];
		if let Some(sigs) = signatures {
			let sig_tokens = signature_params_to_tokens(sigs);
			tokens.extend(sig_tokens);
		}
		tokens
	}
}

/// Outgoing request for migrating the bridge. For the full migration process description see
/// `OutgoingPrepareForMigration` request.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct OutgoingMigrate<T: Config> {
	pub author: T::AccountId,
	pub new_contract_address: EthAddress,
	pub erc20_native_tokens: Vec<EthAddress>,
	pub nonce: T::Nonce,
	pub network_id: BridgeNetworkId<T>,
	pub timepoint: BridgeTimepoint<T>,
	pub new_signature_version: BridgeSignatureVersion,
}

impl<T: Config> OutgoingMigrate<T> {
	pub fn to_eth_abi(&self, tx_hash: H256) -> Result<OutgoingMigrateEncoded, Error<T>> {
		let tx_hash = H256(tx_hash.0);
		let mut network_id: H256 = H256::default();
		U256::from(
			<T::NetworkId as TryInto<u128>>::try_into(self.network_id)
				.ok()
				.expect("NetworkId can be always converted to u128; qed"),
		)
		.to_big_endian(&mut network_id.0);
		let contract_address: EthAddress = crate::BridgeContractAddress::<T>::get(&self.network_id);
		let signature_version = BridgeSignatureVersions::<T>::get(self.network_id);
		let raw = match signature_version {
			BridgeSignatureVersion::V3 => ethabi::encode(&[
				Token::String("migrate".into()),
				Token::Address(types::EthAddress::from(contract_address.0)),
				Token::Address(types::EthAddress::from(self.new_contract_address.0)),
				Token::FixedBytes(tx_hash.0.to_vec()),
				Token::Array(
					self.erc20_native_tokens
						.iter()
						.map(|addr| Token::Address(types::EthAddress::from(addr.0)))
						.collect(),
				),
				Token::FixedBytes(network_id.0.to_vec()),
			]),
		};
		Ok(OutgoingMigrateEncoded {
			this_contract_address: contract_address,
			tx_hash,
			new_contract_address: self.new_contract_address,
			erc20_native_tokens: self.erc20_native_tokens.clone(),
			network_id,
			raw,
		})
	}

	pub fn validate(&self) -> Result<(), DispatchError> {
		ensure!(
			crate::BridgeStatuses::<T>::get(self.network_id).ok_or(Error::<T>::UnknownNetwork)?
				== BridgeStatus::Migrating,
			Error::<T>::ContractIsNotInMigrationStage
		);
		Ok(())
	}

	pub fn prepare(&self, _validated_state: ()) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn cancel(&self) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn finalize(&self) -> Result<(), DispatchError> {
		self.validate()?;
		crate::PendingBridgeSignatureVersions::<T>::insert(
			self.network_id,
			self.new_signature_version,
		);
		Ok(())
	}
}

/// Sidechain-compatible version of `OutgoingMigrate`.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OutgoingMigrateEncoded {
	pub this_contract_address: EthAddress,
	pub tx_hash: H256,
	pub new_contract_address: EthAddress,
	pub erc20_native_tokens: Vec<EthAddress>,
	pub network_id: H256,
	/// EABI-encoded data to be signed.
	pub raw: Vec<u8>,
}

impl OutgoingMigrateEncoded {
	pub fn input_tokens(&self, signatures: Option<Vec<SignatureParams>>) -> Vec<Token> {
		let mut tokens = vec![Token::FixedBytes(self.tx_hash.0.to_vec())];
		if let Some(sigs) = signatures {
			let sig_tokens = signature_params_to_tokens(sigs);
			tokens.extend(sig_tokens);
		}
		tokens
	}
}

/// A helper structure used to add or remove peer on Ethereum network.
///
/// On Ethereum network there are 3 bridge contracts: Main, XOR and VAL. Each of them has a set of
/// peers' public keys that's need to be almost the same at any time (+- 1 signatory). To
/// synchronize them, we use this structure, that contains the current readiness state of each
/// contract. We add or remove peer only when all of them is in `true` state
/// (see `EthPeersSync::is_ready`).
#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct EthPeersSync {
	is_bridge_ready: bool,
	is_xor_ready: bool,
	is_val_ready: bool,
}

impl EthPeersSync {
	pub fn is_ready(&self) -> bool {
		self.is_bridge_ready && self.is_xor_ready && self.is_val_ready
	}

	pub fn bridge_ready(&mut self) {
		self.is_bridge_ready = true;
	}

	pub fn xor_ready(&mut self) {
		self.is_xor_ready = true;
	}

	pub fn val_ready(&mut self) {
		self.is_val_ready = true;
	}

	pub fn reset(&mut self) {
		self.is_val_ready = false;
		self.is_xor_ready = false;
		self.is_bridge_ready = false;
	}
}

/// Parses a `tx_hash` argument of a contract call. `tx_hash` is usually a hash of a Thischain's
/// outgoing request (`OutgoingRequest`).
pub fn parse_hash_from_call<T: Config>(
	tokens: Vec<Token>,
	tx_hash_arg_pos: usize,
) -> Result<H256, Error<T>> {
	tokens
		.get(tx_hash_arg_pos)
		.cloned()
		.and_then(Decoder::<T>::parse_h256)
		.ok_or_else(|| Error::<T>::FailedToParseTxHashInCall.into())
}

macro_rules! impl_from_for_outgoing_requests {
    ($($req:ty, $var:ident);+ $(;)?) => {$(
        impl<T: Config> From<$req> for OutgoingRequest<T> {
            fn from(v: $req) -> Self {
                Self::$var(v)
            }
        }

        impl<T: Config> From<$req> for OffchainRequest<T> {
            fn from(v: $req) -> Self {
                Self::outgoing(v.into())
            }
        }
    )+};
}

impl_from_for_outgoing_requests! {
	OutgoingTransfer<T>, Transfer;
	OutgoingAddPeer<T>, AddPeer;
	OutgoingRemovePeer<T>, RemovePeer;
	OutgoingPrepareForMigration<T>, PrepareForMigration;
	OutgoingMigrate<T>, Migrate;
}
