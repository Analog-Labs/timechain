//! Setup code for [`super::command`] which would otherwise bloat that module.
//!
//! Should only be used for benchmarking as it may break in other contexts.

use crate::service::FullClient;

use polkadot_sdk::*;

use frame_support::dispatch::{DispatchInfo, PostDispatchInfo};
use frame_support::traits::Get;

use frame_system_rpc_runtime_api::AccountNonceApi;

use sc_cli::Result;
use sc_client_api::BlockBackend;

use sp_api::ProvideRuntimeApi;
use sp_consensus_babe::SlotDuration;
use sp_core::crypto::Pair;
use sp_inherents::{InherentData, InherentDataProvider};
use sp_keyring::Sr25519Keyring;
use sp_runtime::codec::Encode;
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_runtime::OpaqueExtrinsic;
use sp_runtime::{generic, SaturatedConversion};

use std::marker::PhantomData;
use std::{sync::Arc, time::Duration};
use time_primitives::{AccountId, Balance, Block, BlockHash, Nonce, Signature};
use timechain_runtime::{
	Address, BalancesCall, PaymentBalanceOf, SignedExtra, SystemCall, SLOT_DURATION,
};

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic<Runtime, Call> =
	sp_runtime::generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra<Runtime>>;
/// The payload being signed in transactions.
pub type SignedPayload<Runtime, Call> =
	sp_runtime::generic::SignedPayload<Call, SignedExtra<Runtime>>;

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
	Runtime:
		frame_system::Config<Hash = BlockHash> + pallet_transaction_payment::Config + Send + Sync,
	Runtime::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	PaymentBalanceOf<Runtime>: From<u64>,
	RuntimeApi: sp_api::ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>,
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
	pub fn new(client: Arc<FullClient<RuntimeApi>>, dest: AccountId, value: Balance) -> Self {
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
	Runtime: frame_system::Config<Hash = BlockHash>
		+ pallet_balances::Config<Balance = Balance>
		+ pallet_transaction_payment::Config
		+ Send
		+ Sync,
	Runtime::Lookup: StaticLookup<Source = Address>,
	Runtime::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>
		+ From<BalancesCall<Runtime>>,
	PaymentBalanceOf<Runtime>: From<u64>,
	RuntimeApi: sp_api::ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>,
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
) -> u32
where
	RuntimeApi: sp_api::ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>,
{
	let best_hash = client.chain_info().best_hash;
	client
		.runtime_api()
		.account_nonce(best_hash, account.public().into())
		.expect("Fetching account nonce works; qed")
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
) -> UncheckedExtrinsic<Runtime, Runtime::RuntimeCall>
where
	Runtime:
		frame_system::Config<Hash = BlockHash> + Send + Sync + pallet_transaction_payment::Config,
	Runtime::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	PaymentBalanceOf<Runtime>: From<u64>,
	RuntimeApi: sp_api::ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>,
{
	let function = function.into();
	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let best_hash = client.chain_info().best_hash;
	let best_block = client.chain_info().best_number;
	let nonce = nonce.unwrap_or_else(|| fetch_nonce::<RuntimeApi>(client, sender.clone()));

	let period = Runtime::BlockHashCount::get()
		.into()
		.as_u64()
		.checked_next_power_of_two()
		.map(|c| c / 2)
		.unwrap_or(2);
	let era = generic::Era::mortal(period, best_block.saturated_into());

	let extra: SignedExtra<Runtime> = (
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckEra::<Runtime>::from(era),
		frame_system::CheckNonce::<Runtime>::from(nonce.into()),
		frame_system::CheckWeight::<Runtime>::new(),
		pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(Default::default()),
		frame_metadata_hash_extension::CheckMetadataHash::new(false),
	);
	let version = Runtime::Version::get();
	let raw_payload = SignedPayload::<Runtime, Runtime::RuntimeCall>::from_raw(
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
	UncheckedExtrinsic::<Runtime, Runtime::RuntimeCall>::new_signed(
		function,
		AccountId::from(sender.public()).into(),
		Signature::Sr25519(signature),
		extra,
	)
}

/// Generates inherent data for the `benchmark overhead` command.
pub fn inherent_benchmark_data() -> Result<InherentData> {
	let mut inherent_data = InherentData::new();

	let timestamp = sp_timestamp::InherentDataProvider::new(Duration::from_millis(0).into());
	futures::executor::block_on(timestamp.provide_inherent_data(&mut inherent_data))
		.map_err(|e| format!("creating time inherent data: {:?}", e))?;

	let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
		*timestamp,
		SlotDuration::from_millis(SLOT_DURATION),
	);
	futures::executor::block_on(slot.provide_inherent_data(&mut inherent_data))
		.map_err(|e| format!("creating slot inherent data: {:?}", e))?;

	Ok(inherent_data)
}
