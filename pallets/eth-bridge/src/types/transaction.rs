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

use crate::types::{Bytes, Index, Log, H160, H2048, H256, U256, U64};
use polkadot_sdk::*;
use serde::{Deserialize, Serialize};
use sp_std::prelude::*;

/// Description of a Transaction, pending or in the chain.
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
	/// Hash
	pub hash: H256,
	/// Nonce
	pub nonce: U256,
	/// Block hash. None when pending.
	pub block_hash: Option<H256>,
	/// Block number. None when pending.
	pub block_number: Option<U64>,
	/// Transaction Index. None when pending.
	pub transaction_index: Option<Index>,
	/// Sender
	pub from: H160,
	/// Recipient (None when contract creation)
	pub to: Option<H160>,
	/// Transfered value
	pub value: U256,
	/// Gas Price
	pub gas_price: U256,
	/// Gas amount
	pub gas: U256,
	/// Input data
	pub input: Bytes,
	/// Raw transaction data
	#[serde(default)]
	pub raw: Option<Bytes>,
}

/// "Receipt" of an executed transaction: details of its execution.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
	/// Transaction hash.
	pub transaction_hash: H256,
	/// Index within the block.
	pub transaction_index: Index,
	/// Hash of the block this transaction was included within.
	pub block_hash: Option<H256>,
	/// Number of the block this transaction was included within.
	pub block_number: Option<U64>,
	/// Sender.
	pub from: H160,
	/// Recipient. `None` if contract deployment.
	pub to: Option<H160>,
	/// Cumulative gas used within the block after this was executed.
	pub cumulative_gas_used: U256,
	/// Gas used by this transaction alone.
	///
	/// Gas used is `None` if the the client is running in light client mode.
	pub gas_used: Option<U256>,
	/// Contract address created, or `None` if not a deployment.
	pub contract_address: Option<H160>,
	/// Logs generated within this transaction.
	pub logs: Vec<Log>,
	/// Status: either 1 (success) or 0 (failure).
	pub status: Option<U64>,
	/// State root.
	pub root: Option<H256>,
	/// Logs bloom
	pub logs_bloom: H2048,
}

impl Receipt {
	pub fn is_approved(&self) -> bool {
		// TODO: handle `root` field?
		self.status.unwrap_or_else(|| 0.into()) != 0.into()
	}
}

/// Raw bytes of a signed, but not yet sent transaction
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawTransaction {
	/// Signed transaction as raw bytes
	pub raw: Bytes,
	/// Transaction details
	pub tx: RawTransactionDetails,
}

/// Details of a signed transaction
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTransactionDetails {
	/// Hash
	pub hash: H256,
	/// Nonce
	pub nonce: U256,
	/// Block hash. None when pending.
	pub block_hash: Option<H256>,
	/// Block number. None when pending.
	pub block_number: Option<U64>,
	/// Transaction Index. None when pending.
	pub transaction_index: Option<Index>,
	/// Sender
	pub from: Option<H160>,
	/// Recipient (None when contract creation)
	pub to: Option<H160>,
	/// Transfered value
	pub value: U256,
	/// Gas Price
	pub gas_price: U256,
	/// Gas amount
	pub gas: U256,
	/// Input data
	pub input: Bytes,
	/// ECDSA recovery id, set by Geth
	pub v: Option<U64>,
	/// ECDSA signature r, 32 bytes, set by Geth
	pub r: Option<U256>,
	/// ECDSA signature s, 32 bytes, set by Geth
	pub s: Option<U256>,
}

#[cfg(test)]
mod tests {
	use super::{RawTransaction, Receipt};
	use serde_json;

	#[test]
	fn test_deserialize_receipt() {
		let receipt_str = "{\"blockHash\":\"0x83eaba432089a0bfe99e9fc9022d1cfcb78f95f407821be81737c84ae0b439c5\",\"blockNumber\":\"0x38\",\"from\":\"0x03d8c4566478a6e1bf75650248accce16a98509f\",\"contractAddress\":\"0x03d8c4566478a6e1bf75650248accce16a98509f\",\"cumulativeGasUsed\":\"0x927c0\",\"gasUsed\":\"0x927c0\",\"logs\":[],\"logsBloom\":\"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\",\"root\":null,\"transactionHash\":\"0x422fb0d5953c0c48cbb42fb58e1c30f5e150441c68374d70ca7d4f191fd56f26\",\"transactionIndex\":\"0x0\"}";

		let _receipt: Receipt = serde_json::from_str(receipt_str).unwrap();
	}

	#[test]
	fn should_deserialize_receipt_with_status() {
		let receipt_str = r#"{
        "blockHash": "0x83eaba432089a0bfe99e9fc9022d1cfcb78f95f407821be81737c84ae0b439c5",
        "blockNumber": "0x38",
        "from": "0x03d8c4566478a6e1bf75650248accce16a98509f",
        "contractAddress": "0x03d8c4566478a6e1bf75650248accce16a98509f",
        "cumulativeGasUsed": "0x927c0",
        "gasUsed": "0x927c0",
        "logs": [],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "root": null,
        "transactionHash": "0x422fb0d5953c0c48cbb42fb58e1c30f5e150441c68374d70ca7d4f191fd56f26",
        "transactionIndex": "0x0",
        "status": "0x1"
    }"#;

		let _receipt: Receipt = serde_json::from_str(receipt_str).unwrap();
	}

	#[test]
	fn should_deserialize_receipt_without_gas() {
		let receipt_str = r#"{
        "blockHash": "0x83eaba432089a0bfe99e9fc9022d1cfcb78f95f407821be81737c84ae0b439c5",
        "blockNumber": "0x38",
        "from": "0x03d8c4566478a6e1bf75650248accce16a98509f",
        "contractAddress": "0x03d8c4566478a6e1bf75650248accce16a98509f",
        "cumulativeGasUsed": "0x927c0",
        "gasUsed": null,
        "logs": [],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "root": null,
        "transactionHash": "0x422fb0d5953c0c48cbb42fb58e1c30f5e150441c68374d70ca7d4f191fd56f26",
        "transactionIndex": "0x0",
        "status": "0x1"
    }"#;

		let _receipt: Receipt = serde_json::from_str(receipt_str).unwrap();
	}

	#[test]
	fn test_deserialize_signed_tx_parity() {
		// taken from RPC docs.
		let tx_str = r#"{
        "raw": "0xd46e8dd67c5d32be8d46e8dd67c5d32be8058bb8eb970870f072445675058bb8eb970870f072445675",
        "tx": {
          "hash": "0xc6ef2fc5426d6ad6fd9e2a26abeab0aa2411b7ab17f30a99d3cb96aed1d1055b",
          "nonce": "0x0",
          "blockHash": "0xbeab0aa2411b7ab17f30a99d3cb9c6ef2fc5426d6ad6fd9e2a26a6aed1d1055b",
          "blockNumber": "0x15df",
          "transactionIndex": "0x1",
          "from": "0x407d73d8a49eeb85d32cf465507dd71d507100c1",
          "to": "0x853f43d8a49eeb85d32cf465507dd71d507100c1",
          "value": "0x7f110",
          "gas": "0x7f110",
          "gasPrice": "0x09184e72a000",
          "input": "0x603880600c6000396000f300603880600c6000396000f3603880600c6000396000f360",
          "s": "0x777"
        }
    }"#;

		let _tx: RawTransaction = serde_json::from_str(tx_str).unwrap();
	}

	#[test]
	fn test_deserialize_signed_tx_geth() {
		let tx_str = r#"{
        "raw": "0xf85d01018094f3b3138e5eb1c75b43994d1bb760e2f9f735789680801ca06484d00575e961a7db35ebe5badaaca5cb7ee65d1f2f22f22da87c238b99d30da07a85d65797e4b555c1d3f64beebb2cb6f16a6fbd40c43cc48451eaf85305f66e",
        "tx": {
          "gas": "0x0",
          "gasPrice": "0x1",
          "hash": "0x0a32fb4e18bc6f7266a164579237b1b5c74271d453c04eab70444ca367d38418",
          "input": "0x",
          "nonce": "0x1",
          "to": "0xf3b3138e5eb1c75b43994d1bb760e2f9f7357896",
          "r": "0x6484d00575e961a7db35ebe5badaaca5cb7ee65d1f2f22f22da87c238b99d30d",
          "s": "0x7a85d65797e4b555c1d3f64beebb2cb6f16a6fbd40c43cc48451eaf85305f66e",
          "v": "0x1c",
          "value": "0x0"
        }
    }"#;

		let _tx: RawTransaction = serde_json::from_str(tx_str).unwrap();
	}
}
