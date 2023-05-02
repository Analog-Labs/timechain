---
title: Runtime
---

The following section contains known runtime calls that may be available on specific runtimes (depending on configuration and available pallets). These call directly into the WASM runtime for queries and operations.

- **[accountNonceApi](#accountnonceapi)**

- **[babeApi](#babeapi)**

- **[blockBuilder](#blockbuilder)**

- **[core](#core)**

- **[grandpaApi](#grandpaapi)**

- **[metadata](#metadata)**

- **[offchainWorkerApi](#offchainworkerapi)**

- **[sessionKeys](#sessionkeys)**

- **[taggedTransactionQueue](#taggedtransactionqueue)**

- **[transactionPaymentApi](#transactionpaymentapi)**

- **[transactionPaymentCallApi](#transactionpaymentcallapi)**


___


## AccountNonceApi
 
### accountNonce(accountId: `AccountId`): `Index`
- **interface**: `api.call.accountNonceApi.accountNonce`
- **runtime**: `AccountNonceApi_account_nonce`
- **summary**: The API to query account nonce (aka transaction index)

___


## BabeApi
 
### configuration(): `BabeGenesisConfiguration`
- **interface**: `api.call.babeApi.configuration`
- **runtime**: `BabeApi_configuration`
- **summary**: Return the genesis configuration for BABE. The configuration is only read on genesis.
 
### currentEpoch(): `Epoch`
- **interface**: `api.call.babeApi.currentEpoch`
- **runtime**: `BabeApi_current_epoch`
- **summary**: Returns information regarding the current epoch.
 
### currentEpochStart(): `Slot`
- **interface**: `api.call.babeApi.currentEpochStart`
- **runtime**: `BabeApi_current_epoch_start`
- **summary**: Returns the slot that started the current epoch.
 
### generateKeyOwnershipProof(slot: `Slot`, authorityId: `AuthorityId`): `Option<OpaqueKeyOwnershipProof>`
- **interface**: `api.call.babeApi.generateKeyOwnershipProof`
- **runtime**: `BabeApi_generate_key_ownership_proof`
- **summary**: Generates a proof of key ownership for the given authority in the current epoch.
 
### nextEpoch(): `Epoch`
- **interface**: `api.call.babeApi.nextEpoch`
- **runtime**: `BabeApi_next_epoch`
- **summary**: Returns information regarding the next epoch (which was already previously announced).
 
### submitReportEquivocationUnsignedExtrinsic(equivocationProof: `BabeEquivocationProof`, keyOwnerProof: `OpaqueKeyOwnershipProof`): `Option<Null>`
- **interface**: `api.call.babeApi.submitReportEquivocationUnsignedExtrinsic`
- **runtime**: `BabeApi_submit_report_equivocation_unsigned_extrinsic`
- **summary**: Submits an unsigned extrinsic to report an equivocation.

___


## BlockBuilder
 
### applyExtrinsic(extrinsic: `Extrinsic`): `ApplyExtrinsicResult`
- **interface**: `api.call.blockBuilder.applyExtrinsic`
- **runtime**: `BlockBuilder_apply_extrinsic`
- **summary**: Apply the given extrinsic.
 
### checkInherents(block: `Block`, data: `InherentData`): `CheckInherentsResult`
- **interface**: `api.call.blockBuilder.checkInherents`
- **runtime**: `BlockBuilder_check_inherents`
- **summary**: Check that the inherents are valid.
 
### finalizeBlock(): `Header`
- **interface**: `api.call.blockBuilder.finalizeBlock`
- **runtime**: `BlockBuilder_finalize_block`
- **summary**: Finish the current block.
 
### inherentExtrinsics(inherent: `InherentData`): `Vec<Extrinsic>`
- **interface**: `api.call.blockBuilder.inherentExtrinsics`
- **runtime**: `BlockBuilder_inherent_extrinsics`
- **summary**: Generate inherent extrinsics.

___


## Core
 
### executeBlock(block: `Block`): `Null`
- **interface**: `api.call.core.executeBlock`
- **runtime**: `Core_execute_block`
- **summary**: Execute the given block.
 
### initializeBlock(header: `Header`): `Null`
- **interface**: `api.call.core.initializeBlock`
- **runtime**: `Core_initialize_block`
- **summary**: Initialize a block with the given header.
 
### version(): `RuntimeVersion`
- **interface**: `api.call.core.version`
- **runtime**: `Core_version`
- **summary**: Returns the version of the runtime.

___


## GrandpaApi
 
### currentSetId(): `SetId`
- **interface**: `api.call.grandpaApi.currentSetId`
- **runtime**: `GrandpaApi_current_set_id`
- **summary**: Get current GRANDPA authority set id.
 
### generateKeyOwnershipProof(setId: `SetId`, authorityId: `AuthorityId`): `Option<OpaqueKeyOwnershipProof>`
- **interface**: `api.call.grandpaApi.generateKeyOwnershipProof`
- **runtime**: `GrandpaApi_generate_key_ownership_proof`
- **summary**: Generates a proof of key ownership for the given authority in the given set.
 
### grandpaAuthorities(): `AuthorityList`
- **interface**: `api.call.grandpaApi.grandpaAuthorities`
- **runtime**: `GrandpaApi_grandpa_authorities`
- **summary**: Get the current GRANDPA authorities and weights. This should not change except for when changes are scheduled and the corresponding delay has passed.
 
### submitReportEquivocationUnsignedExtrinsic(equivocationProof: `GrandpaEquivocationProof`, keyOwnerProof: `OpaqueKeyOwnershipProof`): `Option<Null>`
- **interface**: `api.call.grandpaApi.submitReportEquivocationUnsignedExtrinsic`
- **runtime**: `GrandpaApi_submit_report_equivocation_unsigned_extrinsic`
- **summary**: Submits an unsigned extrinsic to report an equivocation.

___


## Metadata
 
### metadata(): `OpaqueMetadata`
- **interface**: `api.call.metadata.metadata`
- **runtime**: `Metadata_metadata`
- **summary**: Returns the metadata of a runtime
 
### metadataAtVersion(version: `u32`): `Option<OpaqueMetadata>`
- **interface**: `api.call.metadata.metadataAtVersion`
- **runtime**: `Metadata_metadata_at_version`
- **summary**: Returns the metadata at a given version.
 
### metadataVersions(): `Vec<u32>`
- **interface**: `api.call.metadata.metadataVersions`
- **runtime**: `Metadata_metadata_versions`
- **summary**: Returns the supported metadata versions.

___


## OffchainWorkerApi
 
### offchainWorker(header: `Header`): `Null`
- **interface**: `api.call.offchainWorkerApi.offchainWorker`
- **runtime**: `OffchainWorkerApi_offchain_worker`
- **summary**: Starts the off-chain task for given block header.

___


## SessionKeys
 
### decodeSessionKeys(encoded: `Bytes`): `Option<Vec<(Bytes, KeyTypeId)>>`
- **interface**: `api.call.sessionKeys.decodeSessionKeys`
- **runtime**: `SessionKeys_decode_session_keys`
- **summary**: Decode the given public session keys.
 
### generateSessionKeys(seed: `Option<Bytes>`): `Bytes`
- **interface**: `api.call.sessionKeys.generateSessionKeys`
- **runtime**: `SessionKeys_generate_session_keys`
- **summary**: Generate a set of session keys with optionally using the given seed.

___


## TaggedTransactionQueue
 
### validateTransaction(source: `TransactionSource`, tx: `Extrinsic`, blockHash: `BlockHash`): `TransactionValidity`
- **interface**: `api.call.taggedTransactionQueue.validateTransaction`
- **runtime**: `TaggedTransactionQueue_validate_transaction`
- **summary**: Validate the transaction.

___


## TransactionPaymentApi
 
### queryFeeDetails(uxt: `Extrinsic`, len: `u32`): `FeeDetails`
- **interface**: `api.call.transactionPaymentApi.queryFeeDetails`
- **runtime**: `TransactionPaymentApi_query_fee_details`
- **summary**: The transaction fee details
 
### queryInfo(uxt: `Extrinsic`, len: `u32`): `RuntimeDispatchInfo`
- **interface**: `api.call.transactionPaymentApi.queryInfo`
- **runtime**: `TransactionPaymentApi_query_info`
- **summary**: The transaction info
 
### queryLengthToFee(length: `u32`): `Balance`
- **interface**: `api.call.transactionPaymentApi.queryLengthToFee`
- **runtime**: `TransactionPaymentApi_query_length_to_fee`
- **summary**: Query the output of the current LengthToFee given some input
 
### queryWeightToFee(weight: `Weight`): `Balance`
- **interface**: `api.call.transactionPaymentApi.queryWeightToFee`
- **runtime**: `TransactionPaymentApi_query_weight_to_fee`
- **summary**: Query the output of the current WeightToFee given some input

___


## TransactionPaymentCallApi
 
### queryCallFeeDetails(call: `Call`, len: `u32`): `FeeDetails`
- **interface**: `api.call.transactionPaymentCallApi.queryCallFeeDetails`
- **runtime**: `TransactionPaymentCallApi_query_call_fee_details`
- **summary**: The call fee details
 
### queryCallInfo(call: `Call`, len: `u32`): `RuntimeDispatchInfo`
- **interface**: `api.call.transactionPaymentCallApi.queryCallInfo`
- **runtime**: `TransactionPaymentCallApi_query_call_info`
- **summary**: The call info
 
### queryLengthToFee(length: `u32`): `Balance`
- **interface**: `api.call.transactionPaymentCallApi.queryLengthToFee`
- **runtime**: `TransactionPaymentCallApi_query_length_to_fee`
- **summary**: Query the output of the current LengthToFee given some input
 
### queryWeightToFee(weight: `Weight`): `Balance`
- **interface**: `api.call.transactionPaymentCallApi.queryWeightToFee`
- **runtime**: `TransactionPaymentCallApi_query_weight_to_fee`
- **summary**: Query the output of the current WeightToFee given some input
