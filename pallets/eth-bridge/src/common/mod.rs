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

pub mod eth;

use core::marker::PhantomData;

use frame_support::ensure;
use frame_support::traits::Currency;
use frame_support::traits::ExistenceRequirement;
use frame_support::traits::ReservableCurrency;
use frame_support::{
	dispatch::{DispatchErrorWithPostInfo, DispatchResult, DispatchResultWithPostInfo, Pays},
	weights::Weight,
};
use polkadot_sdk::*;
use scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{DispatchError, TransactionOutcome};
use sp_std::prelude::*;

use crate::Config;
use crate::Error;

pub const DEFAULT_BALANCE_PRECISION: u8 = 12;

#[inline(always)]
pub fn pays_no_with_maybe_weight<E: Into<DispatchError>>(
	result: Result<Option<Weight>, E>,
) -> DispatchResultWithPostInfo {
	result
		.map_err(|e| DispatchErrorWithPostInfo {
			post_info: Pays::No.into(),
			error: e.into(),
		})
		.map(|weight| (weight, Pays::No).into())
}

#[inline(always)]
pub fn pays_no<T, E: Into<DispatchError>>(result: Result<T, E>) -> DispatchResultWithPostInfo {
	pays_no_with_maybe_weight(result.map(|_| None))
}

#[inline(always)]
pub fn err_pays_no(err: impl Into<DispatchError>) -> DispatchErrorWithPostInfo {
	DispatchErrorWithPostInfo {
		post_info: Pays::No.into(),
		error: err.into(),
	}
}

pub type Balance = u128;
pub type BalancePrecision = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetId {
	Balances,
}

pub fn with_transaction<T, E>(f: impl FnOnce() -> Result<T, E>) -> Result<T, E>
where
	E: From<sp_runtime::DispatchError>,
{
	frame_support::storage::with_transaction(|| {
		let result = f();
		if result.is_ok() {
			TransactionOutcome::Commit(result)
		} else {
			TransactionOutcome::Rollback(result)
		}
	})
}

pub struct Assets<T>(PhantomData<T>);

impl<T> Assets<T>
where
	T: Config,
{
	pub fn transfer_from(
		asset_id: &AssetId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Balance,
	) -> DispatchResult {
		match asset_id {
			AssetId::Balances => {
				<T as Config>::Currency::transfer(
					&from,
					&to,
					amount,
					ExistenceRequirement::KeepAlive,
				)?;
			},
		}
		Ok(())
	}

	pub fn reserve(asset_id: &AssetId, who: &T::AccountId, amount: Balance) -> DispatchResult {
		match asset_id {
			AssetId::Balances => {
				<T as Config>::Currency::reserve(&who, amount)?;
			},
		}
		Ok(())
	}

	pub fn unreserve(asset_id: &AssetId, who: &T::AccountId, amount: Balance) -> DispatchResult {
		match asset_id {
			AssetId::Balances => {
				let remainder = <T as Config>::Currency::unreserve(&who, amount);
				ensure!(remainder == 0, Error::<T>::FailedToUnreserve);
			},
		}
		Ok(())
	}

	pub fn total_balance(asset_id: &AssetId, who: &T::AccountId) -> Result<Balance, DispatchError> {
		match asset_id {
			AssetId::Balances => Ok(<T as Config>::Currency::total_balance(who)),
		}
	}

	pub fn free_balance(asset_id: &AssetId, who: &T::AccountId) -> Result<Balance, DispatchError> {
		match asset_id {
			AssetId::Balances => Ok(<T as Config>::Currency::free_balance(who)),
		}
	}

	pub fn total_issuance(asset_id: &AssetId) -> Result<Balance, DispatchError> {
		match asset_id {
			AssetId::Balances => Ok(<T as Config>::Currency::total_issuance()),
		}
	}

	// Ensure that we don't leak mint function to production code
	#[cfg(any(test, feature = "runtime-benchmarks"))]
	pub fn mint_to(
		asset_id: &AssetId,
		_caller: &T::AccountId,
		who: &T::AccountId,
		amount: Balance,
	) -> DispatchResult {
		match asset_id {
			AssetId::Balances => {
				let _ = <T as Config>::Currency::deposit_creating(who, amount);
			},
		}
		Ok(())
	}
}

#[cfg(feature = "std")]
pub mod string_serialization {
	use serde::{Deserialize, Deserializer, Serializer};

	pub fn serialize<S: Serializer, T: std::fmt::Display>(
		t: &T,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		serializer.serialize_str(&t.to_string())
	}

	pub fn deserialize<'de, D: Deserializer<'de>, T: std::str::FromStr>(
		deserializer: D,
	) -> Result<T, D::Error> {
		let s = String::deserialize(deserializer)?;
		s.parse::<T>().map_err(|_| serde::de::Error::custom("Parse from string failed"))
	}
}
