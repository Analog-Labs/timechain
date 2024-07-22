//! Setup code for [`super::command`] which would otherwise bloat that module.
//!
//! Should only be used for benchmarking as it may break in other contexts.

use crate::service::FullClient;

use polkadot_sdk::*;

use frame_support::traits::Get;

use sc_cli::Result;
use sp_inherents::{InherentData, InherentDataProvider};
use sp_keyring::Sr25519Keyring;
use sp_runtime::OpaqueExtrinsic;

use sc_client_api::BlockBackend;
use sp_core::crypto::Pair;
use sp_runtime::codec::Encode;
use sp_runtime::{generic, SaturatedConversion};

use runtime_common::{BalancesCall, SystemCall};
use time_primitives::{AccountId, Balance, Block, Nonce};

use std::marker::PhantomData;
use std::{sync::Arc, time::Duration};

/// Generates `System::Remark` extrinsics for the benchmarks.
///
/// Note: Should only be used for benchmarking.
pub struct RemarkBuilder<Runtime, RuntimeApi> {
	client: Arc<FullClient<RuntimeApi>>,
	_runtime: PhantomData<Runtime>,
}

impl<Runtime, RuntimeApi> RemarkBuilder<Runtime, RuntimeApi> {
	/// Creates a new [`Self`] from the given client.
	pub fn new(client: Arc<FullClient<RuntimeApi>>) -> Self {
		Self { client, _runtime: PhantomData }
	}
}

impl<Runtime, RuntimeApi> frame_benchmarking_cli::ExtrinsicBuilder
	for RemarkBuilder<Runtime, RuntimeApi>
where
	Runtime: frame_system::Config + pallet_transaction_payment::Config + Send + Sync,
	RuntimeApi: sp_api::ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
{
	fn pallet(&self) -> &str {
		"system"
	}

	fn extrinsic(&self) -> &str {
		"remark"
	}

	fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
		let acc = Sr25519Keyring::Bob.pair();
		let extrinsic: OpaqueExtrinsic = create_extrinsic::<Runtime, RuntimeApi>(
			self.client.as_ref(),
			acc,
			SystemCall::remark { remark: vec![] },
			Some(nonce),
		)
		.into();

		Ok(extrinsic)
	}
}

/// Generates `Balances::TransferKeepAlive` extrinsics for the benchmarks.
///
/// Note: Should only be used for benchmarking.
pub struct TransferKeepAliveBuilder<Runtime, RuntimeApi> {
	client: Arc<FullClient<RuntimeApi>>,
	dest: AccountId,
	value: Balance,
	_runtime: PhantomData<Runtime>,
}

impl<Runtime, RuntimeApi> TransferKeepAliveBuilder<Runtime, RuntimeApi> {
	/// Creates a new [`Self`] from the given client.
	pub fn new(client: Arc<FullClient<RuntimeApi>>, dest: AccountId) -> Self {
		let value = 0;
		//value = client.constants.balances.existential_deposit()
		Self {
			client,
			dest,
			value,
			_runtime: PhantomData,
		}
	}
}

impl<Runtime, RuntimeApi> frame_benchmarking_cli::ExtrinsicBuilder
	for TransferKeepAliveBuilder<Runtime, RuntimeApi>
where
	Runtime: frame_system::Config + pallet_transaction_payment::Config + Send + Sync,
	RuntimeApi: sp_api::ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
{
	fn pallet(&self) -> &str {
		"balances"
	}

	fn extrinsic(&self) -> &str {
		"transfer_keep_alive"
	}

	fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
		let acc = Sr25519Keyring::Bob.pair();
		let extrinsic: OpaqueExtrinsic = create_extrinsic::<Runtime, RuntimeApi>(
			self.client.as_ref(),
			acc,
			BalancesCall::transfer_keep_alive {
				dest: self.dest.clone().into(),
				value: self.value,
			},
			Some(nonce),
		)
		.into();

		Ok(extrinsic)
	}
}

/// Fetch the nonce of the given `account` from the chain state.
///
/// Note: Should only be used for tests.
pub fn fetch_nonce<RuntimeApi>(
	client: &FullClient<RuntimeApi>,
	account: sp_core::sr25519::Pair,
) -> u32 {
	let best_hash = client.chain_info().best_hash;
	//client
	//	.runtime_api()
	//	.account_nonce(best_hash, account.public().into())
	//	.expect("Fetching account nonce works; qed")
	0
}

/// Create a transaction using the given `call`.
///
/// The transaction will be signed by `sender`. If `nonce` is `None` it will be fetched from the
/// state of the best block.
///
/// Note: Should only be used for tests.
pub fn create_extrinsic<Runtime, RuntimeApi>(
	client: &FullClient<RuntimeApi>,
	sender: sp_core::sr25519::Pair,
	function: impl Into<Runtime::RuntimeCall>,
	nonce: Option<u32>,
) -> runtime_common::UncheckedExtrinsic<Runtime, Runtime::RuntimeCall>
where
	Runtime: frame_system::Config + Send + Sync + pallet_transaction_payment::Config,
	//+ frame_support::dispatch::GetDispatchInfo,
	RuntimeApi: sp_api::ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
{
	let function = function.into();
	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let best_hash = client.chain_info().best_hash;
	let best_block = client.chain_info().best_number;
	let nonce = nonce.unwrap_or_else(|| fetch_nonce::<RuntimeApi>(client, sender.clone()));

	let period = Runtime::BlockHashCount::get()
		.checked_next_power_of_two()
		.map(|c| c / 2)
		.unwrap_or(2) as u64;
	let tip = 0;
	let extra: runtime_common::SignedExtra<Runtime> = (
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(
			period,
			best_block.saturated_into(),
		)),
		frame_system::CheckNonce::<Runtime>::from(nonce.into()),
		frame_system::CheckWeight::<Runtime>::new(),
		pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip.into()),
		frame_metadata_hash_extension::CheckMetadataHash::new(false),
	);

	let version = Runtime::version();

	let raw_payload = runtime_common::SignedPayload::<Runtime, Runtime::RuntimeCall>::from_raw(
		function.clone(),
		extra.clone(),
		(
			(),
			version.spec_version,
			version.transaction_version,
			genesis_hash,
			best_hash,
			(),
			(),
			(),
			None,
		),
	);
	let signature = raw_payload.using_encoded(|e| sender.sign(e));

	runtime_common::UncheckedExtrinsic::<Runtime, Runtime::RuntimeCall>::new_signed(
		function,
		sp_runtime::AccountId32::from(sender.public()).into(),
		time_primitives::Signature::Sr25519(signature),
		extra,
	)
}

/// Generates inherent data for the `benchmark overhead` command.
pub fn inherent_benchmark_data() -> Result<InherentData> {
	let mut inherent_data = InherentData::new();
	let d = Duration::from_millis(0);
	let timestamp = sp_timestamp::InherentDataProvider::new(d.into());

	futures::executor::block_on(timestamp.provide_inherent_data(&mut inherent_data))
		.map_err(|e| format!("creating inherent data: {:?}", e))?;
	Ok(inherent_data)
}
