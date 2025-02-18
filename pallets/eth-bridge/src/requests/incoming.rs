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

#[cfg(feature = "std")]
use crate::common::string_serialization;
use crate::common::{AssetId, Assets, Balance};
use crate::contract::{MethodId, FUNCTIONS, METHOD_ID_SIZE};
use crate::offchain::SignatureParams;
use crate::types::EthAddress;
use crate::util::get_bridge_account;
use crate::{
	AssetKind, BridgeNetworkId, BridgeStatus, BridgeTimepoint, Config, Error, OffchainRequest,
	OutgoingRequest, RequestStatus, Timepoint,
};
use alloc::collections::BTreeSet;
#[allow(unused_imports)]
use frame_support::debug;
use frame_support::dispatch::DispatchResult;
use frame_support::ensure;
use frame_support::sp_runtime::app_crypto::sp_core;
use polkadot_sdk::*;
use scale_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::RuntimeDebug;
use sp_core::H256;
use sp_runtime::DispatchError;
use sp_std::prelude::*;

pub const MIN_PEERS: usize = 4;
pub const MAX_PEERS: usize = 100;

/// Incoming request for adding/removing peer in a bridge.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct IncomingChangePeers<T: Config> {
	pub peer_account_id: Option<T::AccountId>,
	pub peer_address: EthAddress,
	pub removed: bool,
	pub author: T::AccountId,
	pub tx_hash: H256,
	pub at_height: u64,
	pub timepoint: BridgeTimepoint<T>,
	pub network_id: BridgeNetworkId<T>,
}

impl<T: Config> IncomingChangePeers<T> {
	/// Checks if the pending peer matches with a peer in the request, then adds a signatory to the
	/// bridge multisig account and to the peers set (if `added` is true), otherwise does nothing,
	/// because the peer was removed early (in the corresponding outgoing request). Finally, it
	/// cleans the pending peer value.
	pub fn finalize(&self) -> Result<H256, DispatchError> {
		let pending_peer =
			crate::PendingPeer::<T>::get(self.network_id).ok_or(Error::<T>::NoPendingPeer)?;
		if !self.removed {
			ensure!(
				&pending_peer
					== self.peer_account_id.as_ref().ok_or(Error::<T>::UnknownPeerAddress)?,
				Error::<T>::WrongPendingPeer
			);
		}
		if self.removed {
			if let Some(peer) = &self.peer_account_id {
				frame_system::Pallet::<T>::dec_consumers(peer);
			}
		} else {
			let account_id =
				self.peer_account_id.as_ref().ok_or(Error::<T>::UnknownPeerAddress)?.clone();
			bridge_multisig::Pallet::<T>::add_signatory(
				get_bridge_account::<T>(self.network_id),
				account_id.clone(),
			)?;
			crate::Peers::<T>::mutate(self.network_id, |set| set.insert(account_id));
		}
		crate::PendingPeer::<T>::take(self.network_id);
		Ok(self.tx_hash)
	}

	pub fn timepoint(&self) -> Timepoint<T> {
		self.timepoint
	}

	pub fn author(&self) -> &T::AccountId {
		&self.author
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ChangePeersContract {
	XOR,
	VAL,
}

/// Incoming request for transferring token from Sidechain to Thischain.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct IncomingTransfer<T: Config> {
	pub from: EthAddress,
	pub to: T::AccountId,
	pub asset_id: AssetId,
	pub asset_kind: AssetKind,
	#[cfg_attr(feature = "std", serde(with = "string_serialization"))]
	pub amount: Balance,
	pub author: T::AccountId,
	pub tx_hash: H256,
	pub at_height: u64,
	pub timepoint: BridgeTimepoint<T>,
	pub network_id: BridgeNetworkId<T>,
	pub should_take_fee: bool,
}

impl<T: Config> IncomingTransfer<T> {
	pub fn validate(&self) -> Result<(), DispatchError> {
		Ok(())
	}

	/// If the asset kind is owned, then the `amount` of funds is reserved on the bridge account.
	pub fn prepare(&self) -> Result<(), DispatchError> {
		let bridge_account = get_bridge_account::<T>(self.network_id);
		Assets::<T>::reserve(&self.asset_id, &bridge_account, self.amount)?;
		Ok(())
	}

	/// Unreserves previously reserved amount of funds if the asset kind is owned.
	pub fn unreserve(&self) -> DispatchResult {
		let bridge_acc = &get_bridge_account::<T>(self.network_id);
		Assets::<T>::unreserve(&self.asset_id, bridge_acc, self.amount)?;
		Ok(())
	}

	/// Calls `.unreserve`.
	pub fn cancel(&self) -> Result<(), DispatchError> {
		self.unreserve()
	}

	/// If the transferring asset kind is owned, the funds are transferred from the bridge account,
	/// otherwise the amount is minted.
	pub fn finalize(&self) -> Result<H256, DispatchError> {
		self.validate()?;
		let bridge_account_id = get_bridge_account::<T>(self.network_id);
		crate::common::with_transaction(|| -> Result<_, DispatchError> {
			self.unreserve()?;
			Assets::<T>::transfer_from(&self.asset_id, &bridge_account_id, &self.to, self.amount)?;
			Ok(())
		})?;
		Ok(self.tx_hash)
	}

	pub fn timepoint(&self) -> Timepoint<T> {
		self.timepoint
	}

	pub fn author(&self) -> &T::AccountId {
		&self.author
	}

	pub fn enable_taking_fee(&mut self) {
		self.should_take_fee = true;
	}
}

/// Encodes the given outgoing request as it should look when it gets called on Sidechain.
pub fn encode_outgoing_request_eth_call<T: Config>(
	method_id: MethodId,
	request: &OutgoingRequest<T>,
	request_hash: H256,
) -> Result<Vec<u8>, Error<T>> {
	let fun_metas = &FUNCTIONS.get().unwrap();
	let fun_meta = fun_metas.get(&method_id).ok_or(Error::UnknownMethodId)?;
	let request_encoded = request.to_eth_abi(request_hash)?;
	let approvals: BTreeSet<SignatureParams> =
		crate::RequestApprovals::<T>::get(request.network_id(), &request_hash);
	let input_tokens = request_encoded.input_tokens(Some(approvals.into_iter().collect()));
	fun_meta
		.function
		.encode_input(&input_tokens)
		.map_err(|_| Error::EthAbiEncodingError)
}

/// Incoming request for cancelling a obsolete outgoing request. "Obsolete" means that the request
/// signatures were collected, but something changed in the bridge state (e.g., peers set) and
/// the signatures became invalid. In this case we want to cancel the request to be able to
/// re-submit it later.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct IncomingCancelOutgoingRequest<T: Config> {
	pub outgoing_request: OutgoingRequest<T>,
	pub outgoing_request_hash: H256,
	pub initial_request_hash: H256,
	pub tx_input: Vec<u8>,
	pub author: T::AccountId,
	pub tx_hash: H256,
	pub at_height: u64,
	pub timepoint: BridgeTimepoint<T>,
	pub network_id: BridgeNetworkId<T>,
}

impl<T: Config> IncomingCancelOutgoingRequest<T> {
	/// Checks that the request status is `ApprovalsReady`, and encoded request's call matches
	/// with the `tx_input`, otherwise an error is thrown. After that, a status of the request
	/// is changed to `Frozen` to stop receiving approvals.
	pub fn prepare(&self) -> Result<(), DispatchError> {
		let request_hash = self.outgoing_request_hash;
		let net_id = self.network_id;
		let req_status = crate::RequestStatuses::<T>::get(net_id, &self.outgoing_request_hash)
			.ok_or(crate::Error::<T>::UnknownRequest)?;
		ensure!(req_status == RequestStatus::ApprovalsReady, crate::Error::<T>::RequestIsNotReady);
		let mut method_id = [0u8; METHOD_ID_SIZE];
		ensure!(self.tx_input.len() >= 4, Error::<T>::InvalidFunctionInput);
		method_id.clone_from_slice(&self.tx_input[..METHOD_ID_SIZE]);
		let expected_input = encode_outgoing_request_eth_call(
			method_id,
			&self.outgoing_request,
			self.outgoing_request_hash,
		)?;
		ensure!(expected_input == self.tx_input, crate::Error::<T>::InvalidContractInput);
		crate::RequestStatuses::<T>::insert(net_id, &request_hash, RequestStatus::Frozen);
		Ok(())
	}

	/// Changes the request's status back to `ApprovalsReady`.
	pub fn cancel(&self) -> Result<(), DispatchError> {
		crate::RequestStatuses::<T>::insert(
			self.network_id,
			&self.outgoing_request_hash,
			RequestStatus::ApprovalsReady,
		);
		Ok(())
	}

	/// Calls `cancel` on the request, changes its status to `Failed` and takes it approvals to
	/// make it available for resubmission.
	pub fn finalize(&self) -> Result<H256, DispatchError> {
		// TODO: `common::with_transaction` should be removed in the future after stabilization.
		crate::common::with_transaction(|| self.outgoing_request.cancel())?;
		let hash = &self.outgoing_request_hash;
		let net_id = self.network_id;
		crate::RequestStatuses::<T>::insert(
			net_id,
			hash,
			RequestStatus::Failed(Error::<T>::Cancelled.into()),
		);
		crate::RequestApprovals::<T>::take(net_id, hash);
		Ok(self.initial_request_hash)
	}

	pub fn timepoint(&self) -> Timepoint<T> {
		self.timepoint
	}

	pub fn author(&self) -> &T::AccountId {
		&self.author
	}
}

/// Incoming request that's used to mark outgoing requests as done.
/// Since off-chain workers query Sidechain networks lazily, we should force them to check
/// if some outgoing request was finalized on Sidechain.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct IncomingMarkAsDoneRequest<T: Config> {
	pub outgoing_request_hash: H256,
	pub initial_request_hash: H256,
	pub author: T::AccountId,
	pub at_height: u64,
	pub timepoint: BridgeTimepoint<T>,
	pub network_id: BridgeNetworkId<T>,
}

impl<T: Config> IncomingMarkAsDoneRequest<T> {
	/// Checks that the marking request status is `ApprovalsReady`.
	pub fn validate(&self) -> Result<(), DispatchError> {
		let request_status =
			crate::RequestStatuses::<T>::get(self.network_id, self.outgoing_request_hash)
				.ok_or(Error::<T>::UnknownRequest)?;
		ensure!(request_status == RequestStatus::ApprovalsReady, Error::<T>::RequestIsNotReady);
		Ok(())
	}

	pub fn prepare(&self) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn cancel(&self) -> Result<(), DispatchError> {
		Ok(())
	}

	/// Validates the request again and changes the status of the marking request to `Done`.
	pub fn finalize(&self) -> Result<H256, DispatchError> {
		self.validate()?;
		crate::RequestStatuses::<T>::insert(
			self.network_id,
			self.outgoing_request_hash,
			RequestStatus::Done,
		);
		Ok(self.initial_request_hash)
	}

	pub fn timepoint(&self) -> Timepoint<T> {
		self.timepoint
	}

	pub fn author(&self) -> &T::AccountId {
		&self.author
	}
}

/// Incoming request that acts as an acknowledgement to a corresponding
/// `OutgoingPrepareForMigration` request.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct IncomingPrepareForMigration<T: Config> {
	pub author: T::AccountId,
	pub tx_hash: H256,
	pub at_height: u64,
	pub timepoint: BridgeTimepoint<T>,
	pub network_id: BridgeNetworkId<T>,
}

impl<T: Config> IncomingPrepareForMigration<T> {
	/// Checks that the current bridge status is `Initialized`, otherwise an error is thrown.
	pub fn prepare(&self) -> Result<(), DispatchError> {
		ensure!(
			crate::BridgeStatuses::<T>::get(&self.network_id).ok_or(Error::<T>::UnknownNetwork)?
				== BridgeStatus::Initialized,
			Error::<T>::ContractIsAlreadyInMigrationStage
		);
		Ok(())
	}

	pub fn cancel(&self) -> Result<(), DispatchError> {
		Ok(())
	}

	/// Sets the bridge status to `Migrating`.
	pub fn finalize(&self) -> Result<H256, DispatchError> {
		crate::BridgeStatuses::<T>::insert(self.network_id, BridgeStatus::Migrating);
		Ok(self.tx_hash)
	}

	pub fn timepoint(&self) -> Timepoint<T> {
		self.timepoint
	}

	pub fn author(&self) -> &T::AccountId {
		&self.author
	}
}

/// Incoming request that acts as an acknowledgement to a corresponding
/// `OutgoingMigrate` request.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[scale_info(skip_type_params(T))]
pub struct IncomingMigrate<T: Config> {
	pub new_contract_address: EthAddress,
	pub author: T::AccountId,
	pub tx_hash: H256,
	pub at_height: u64,
	pub timepoint: BridgeTimepoint<T>,
	pub network_id: BridgeNetworkId<T>,
}

impl<T: Config> IncomingMigrate<T> {
	/// Checks that the current bridge status is `Migrating`, otherwise an error is thrown.
	pub fn prepare(&self) -> Result<(), DispatchError> {
		ensure!(
			crate::BridgeStatuses::<T>::get(&self.network_id).ok_or(Error::<T>::UnknownNetwork)?
				== BridgeStatus::Migrating
				&& crate::PendingBridgeSignatureVersions::<T>::get(&self.network_id).is_some(),
			Error::<T>::ContractIsNotInMigrationStage
		);
		Ok(())
	}

	pub fn cancel(&self) -> Result<(), DispatchError> {
		Ok(())
	}

	/// Updates the bridge's contract address and sets its status to `Initialized`.
	pub fn finalize(&self) -> Result<H256, DispatchError> {
		crate::BridgeContractAddress::<T>::insert(self.network_id, self.new_contract_address);
		crate::BridgeStatuses::<T>::insert(self.network_id, BridgeStatus::Initialized);
		let signature_version = crate::PendingBridgeSignatureVersions::<T>::take(&self.network_id)
			.ok_or(Error::<T>::ContractIsNotInMigrationStage)?;
		crate::BridgeSignatureVersions::<T>::insert(self.network_id, signature_version);
		Ok(self.tx_hash)
	}

	pub fn timepoint(&self) -> Timepoint<T> {
		self.timepoint
	}

	pub fn author(&self) -> &T::AccountId {
		&self.author
	}
}
macro_rules! impl_from_for_incoming_requests {
    ($($req:ty, $var:ident);+ $(;)?) => {$(
        impl<T: Config> From<$req> for crate::IncomingRequest<T> {
            fn from(v: $req) -> Self {
                Self::$var(v)
            }
        }

        impl<T: Config> From<$req> for OffchainRequest<T> {
            fn from(v: $req) -> Self {
                Self::incoming(v.into())
            }
        }
    )+};
}

impl_from_for_incoming_requests! {
	IncomingTransfer<T>, Transfer;
	IncomingChangePeers<T>, ChangePeers;
	IncomingPrepareForMigration<T>, PrepareForMigration;
	IncomingMigrate<T>, Migrate;
	IncomingCancelOutgoingRequest<T>, CancelOutgoingRequest;
}
