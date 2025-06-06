//! Runtime API Implementation

use polkadot_sdk::*;

use scale_codec::Encode;
#[cfg(feature = "runtime-benchmarks")]
use scale_info::prelude::string::String;

use frame_support::{traits::KeyOwnerProofSystem, weights::Weight};
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::crypto::KeyTypeId;
use sp_core::OpaqueMetadata;
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
	traits::{Block as BlockT, NumberFor},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};
use sp_std::prelude::*;
use sp_version::RuntimeVersion;

#[cfg(feature = "testnet")]
use frame_support::dispatch::DispatchInfo;
#[cfg(feature = "testnet")]
use frame_system::limits::BlockWeights;
#[cfg(feature = "testnet")]
use pallet_revive::{evm::runtime::EthExtra, AddressMapper};
#[cfg(feature = "testnet")]
use sp_core::{H160, U256};
#[cfg(feature = "testnet")]
use sp_runtime::traits::TransactionExtension;

// Primitives imports
pub use time_primitives::{MembersInterface, NetworksInterface};

#[cfg(feature = "testnet")]
use time_primitives::{
	Address32, BatchId, BlockNumber, ChainName, Commitment, ErrorMsg, GatewayMessage, MemberStatus,
	NetworkConfig, NetworkId, PeerId, ShardId, ShardStatus, Task, TaskId,
};

// Local module imports
use super::{
	AccountId, AuthorityDiscovery, Babe, Balance, Block, EpochDuration, Executive, Grandpa,
	Historical, InherentDataExt, NominationPools, Nonce, Runtime, RuntimeCall, SessionKeys,
	Staking, System, TransactionPayment, BABE_GENESIS_EPOCH_CONFIG, VERSION,
};
#[cfg(feature = "genesis-builder")]
use crate::RuntimeGenesisConfig;
#[cfg(feature = "testnet")]
use crate::{
	EthExtraImpl, Members, Networks, Revive, RuntimeBlockWeights, RuntimeOrigin, Shards, Tasks,
	UncheckedExtrinsic,
};

sp_api::impl_runtime_apis! {

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
			sp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			//let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof.decode()?,
			)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			Historical::prove((sp_consensus_grandpa::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_grandpa::OpaqueKeyOwnershipProof::new)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl pallet_nomination_pools_runtime_api::NominationPoolsApi<
		Block,
		AccountId,
		Balance,
	> for Runtime {
		fn pending_rewards(member: AccountId) -> Balance {
			NominationPools::api_pending_rewards(member).unwrap_or_default()
		}

		fn points_to_balance(pool_id: pallet_nomination_pools::PoolId, points: Balance) -> Balance {
			NominationPools::api_points_to_balance(pool_id, points)
		}

		fn balance_to_points(pool_id: pallet_nomination_pools::PoolId, new_funds: Balance) -> Balance {
			NominationPools::api_balance_to_points(pool_id, new_funds)
		}

		fn pool_pending_slash(pool_id: pallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_pending_slash(pool_id)
		}

		fn member_pending_slash(member: AccountId) -> Balance {
			NominationPools::api_member_pending_slash(member)
		}

		fn pool_needs_delegate_migration(pool_id: pallet_nomination_pools::PoolId) -> bool {
			NominationPools::api_pool_needs_delegate_migration(pool_id)
		}

		fn member_needs_delegate_migration(member: AccountId) -> bool {
			NominationPools::api_member_needs_delegate_migration(member)
		}

		fn member_total_balance(who: AccountId) -> Balance {
			NominationPools::api_member_total_balance(who)
		}

		fn pool_balance(pool_id: pallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_balance(pool_id)
		}

		fn pool_accounts(pool_id: pallet_nomination_pools::PoolId) -> (AccountId, AccountId) {
			NominationPools::api_pool_accounts(pool_id)
		}
	}

	impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
		fn nominations_quota(balance: Balance) -> u32 {
			Staking::api_nominations_quota(balance)
		}

		fn eras_stakers_page_count(era: sp_staking::EraIndex, account: AccountId) -> sp_staking::Page {
			Staking::api_eras_stakers_page_count(era, account)
		}

		fn pending_rewards(era: sp_staking::EraIndex, account: AccountId) -> bool {
			Staking::api_pending_rewards(era, account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}

		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}

		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}

		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(call: RuntimeCall, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(call: RuntimeCall, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	// Experimental APIs used only on testnet yet

	#[cfg(feature = "testnet")]
	impl time_primitives::MembersApi<Block> for Runtime {
		fn member_peer_id(account: &AccountId) -> Option<PeerId> {
			Members::member_peer_id(account)
		}

		fn heartbeat_timeout() -> BlockNumber {
			Members::heartbeat_timeout()
		}
	}

	#[cfg(feature = "testnet")]
	impl time_primitives::NetworksApi<Block> for Runtime {
		fn network_name(network_id: NetworkId) -> Option<ChainName> {
			Networks::network_name(network_id)
		}

		fn network_gateway(network: NetworkId) -> Option<Address32> {
			Networks::gateway(network)
		}

		fn network_config(network: NetworkId) -> NetworkConfig {
			Networks::network_config(network)
		}
	}

	#[cfg(feature = "testnet")]
	impl time_primitives::ShardsApi<Block> for Runtime {
		fn shards(account: &AccountId) -> Vec<ShardId> {
			Shards::shards(account)
		}

		fn shard_members(shard_id: ShardId) -> Vec<(AccountId, MemberStatus)> {
			Shards::shard_members(shard_id)
		}

		fn shard_threshold(shard_id: ShardId) -> u16 {
			Shards::shard_threshold(shard_id)
		}

		fn shard_status(shard_id: ShardId) -> ShardStatus {
			Shards::shard_status(shard_id)
		}

		fn shard_commitment(shard_id: ShardId) -> Option<Commitment> {
			Shards::shard_commitment(shard_id)
		}
	}

	#[cfg(feature = "testnet")]
	impl time_primitives::TasksApi<Block> for Runtime {
		fn shard_tasks(shard_id: ShardId) -> Vec<TaskId> {
			Tasks::shard_tasks(shard_id)
		}

		fn task(task_id: TaskId) -> Option<Task>{
			Tasks::task(task_id)
		}

		fn task_result(task_id: TaskId) -> Option<Result<(), ErrorMsg>>{
			Tasks::task_result(task_id)
		}

		fn task_shard(task_id: TaskId) -> Option<ShardId>{
			Tasks::task_shard(task_id)
		}

		fn batch_message(batch_id: BatchId) -> Option<GatewayMessage> {
			Tasks::batch_message(batch_id)
		}

		fn failed_batches() -> Vec<BatchId> {
			Tasks::failed_batches()
		}

		fn pending_batches() -> Vec<BatchId> {
			Tasks::pending_batches()
		}
	}

	#[cfg(feature = "testnet")]
	impl time_primitives::SubmitTransactionApi<Block> for Runtime {
		fn submit_transaction(encoded_transaction: Vec<u8>) -> Result<(), ()> {
			sp_io::offchain::submit_transaction(encoded_transaction)
		}
	}

	#[cfg(feature = "testnet")]
	impl pallet_revive::ReviveApi<Block, AccountId, Balance, Nonce, BlockNumber> for Runtime
	{
		fn balance(address: H160) -> U256 {
			Revive::evm_balance(&address)
		}

		fn block_gas_limit() -> U256 {
			Revive::evm_block_gas_limit()
		}

		fn gas_price() -> U256 {
			Revive::evm_gas_price()
		}

		fn nonce(address: H160) -> Nonce {
			let account = <Runtime as pallet_revive::Config>::AddressMapper::to_account_id(&address);
			System::account_nonce(account)
		}

		fn eth_transact(tx: pallet_revive::evm::GenericTransaction) -> Result<pallet_revive::EthTransactInfo<Balance>, pallet_revive::EthTransactError>
		{
			let blockweights: BlockWeights = <Runtime as frame_system::Config>::BlockWeights::get();
			let tx_fee = |pallet_call, mut dispatch_info: DispatchInfo| {
				let call = RuntimeCall::Revive(pallet_call);
				dispatch_info.extension_weight = EthExtraImpl::get_eth_extension(0, 0u32.into()).weight(&call);
				let uxt: UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic::new_bare(call).into();

				pallet_transaction_payment::Pallet::<Runtime>::compute_fee(
					uxt.encoded_size() as u32,
					&dispatch_info,
					0u32.into(),
				)
			};

			Revive::bare_eth_transact(tx, blockweights.max_block, tx_fee)
		}

		fn call(
			origin: AccountId,
			dest: H160,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_revive::ContractResult<pallet_revive::ExecReturnValue, Balance> {
			Revive::bare_call(
				RuntimeOrigin::signed(origin),
				dest,
				value,
				gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block),
				pallet_revive::DepositLimit::Balance(storage_deposit_limit.unwrap_or(u128::MAX)),
				input_data,
			)
		}

		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			code: pallet_revive::Code,
			data: Vec<u8>,
			salt: Option<[u8; 32]>,
		) -> pallet_revive::ContractResult<pallet_revive::InstantiateReturnValue, Balance>
		{
			Revive::bare_instantiate(
				RuntimeOrigin::signed(origin),
				value,
				gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block),
				pallet_revive::DepositLimit::Balance(storage_deposit_limit.unwrap_or(u128::MAX)),
				code,
				data,
				salt,
			)
		}

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
		) -> pallet_revive::CodeUploadResult<Balance>
		{
			Revive::bare_upload_code(
				RuntimeOrigin::signed(origin),
				code,
				storage_deposit_limit.unwrap_or(u128::MAX),
			)
		}

		fn get_storage(
			address: H160,
			key: [u8; 32],
		) -> pallet_revive::GetStorageResult {
			Revive::get_storage(
				address,
				key
			)
		}

		fn trace_block(
			block: Block,
			tracer_type: pallet_revive::evm::TracerType
		) -> Vec<(u32, pallet_revive::evm::Trace)> {
			use pallet_revive::tracing::trace;
			let mut tracer = Revive::evm_tracer(tracer_type);
			let mut traces = vec![];
			let (header, extrinsics) = block.deconstruct();

			Executive::initialize_block(&header);
			for (index, ext) in extrinsics.into_iter().enumerate() {
				trace(tracer.as_tracing(), || {
					let _ = Executive::apply_extrinsic(ext);
				});

				if let Some(tx_trace) = tracer.collect_trace() {
					traces.push((index as u32, tx_trace));
				}
			}

			traces
		}

		fn trace_tx(
			block: Block,
			tx_index: u32,
			tracer_type: pallet_revive::evm::TracerType
		) -> Option<pallet_revive::evm::Trace> {
			use pallet_revive::tracing::trace;
			let mut tracer = Revive::evm_tracer(tracer_type);
			let (header, extrinsics) = block.deconstruct();

			Executive::initialize_block(&header);
			for (index, ext) in extrinsics.into_iter().enumerate() {
				if index as u32 == tx_index {
					trace(tracer.as_tracing(), || {
					let _ = Executive::apply_extrinsic(ext);
					});
					break;
				} else {
					let _ = Executive::apply_extrinsic(ext);
				}
			}

			tracer.collect_trace()
		}

		fn trace_call(
			tx: pallet_revive::evm::GenericTransaction,
			trace_type: pallet_revive::evm::TracerType)
			-> Result<pallet_revive::evm::Trace, pallet_revive::EthTransactError>
		{
			use pallet_revive::tracing::trace;
			let mut tracer = Revive::evm_tracer(trace_type);
			let result = trace(tracer.as_tracing(), || Self::eth_transact(tx));

			if let Some(trace) = tracer.collect_trace() {
				Ok(trace)
			} else if let Err(err) = result {
				Err(err)
			} else {
				Ok(tracer.empty_trace())
			}
		}
	}

	// Optional runtime interfaces controlled by feature flags

	/// - __genesis-builder__: support generation of custom genesis
	#[cfg(feature = "genesis-builder")]
	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {

		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			use frame_support::genesis_builder_helper::build_state;
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			use frame_support::genesis_builder_helper::get_preset;
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![]
		}
	}

	/// - __runtime-benchmarks__: support runtime benchmarking
	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {

		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use frame_system_benchmarking::extensions::Pallet as SystemExtensionsBench;
			use baseline::Pallet as BaselineBench;

			// Import substrate macros created by macros (and all pallets by effects)
			use crate::*;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);
			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, String> {
			use frame_benchmarking::{baseline, BenchmarkBatch};

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use frame_system_benchmarking::extensions::Pallet as SystemExtensionsBench;
			use baseline::Pallet as BaselineBench;

			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_nomination_pools_benchmarking::Config for Runtime {}
			impl pallet_offences_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}
			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::{TrackedStorageKey, WhitelistedStorageKeys};
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			// Import substrate macros created by macros (and all pallets by effects)
			use crate::*;

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);
			Ok(batches)
		}
	}

	/// - __try-runtime__: support try runtime testing
	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, crate::RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}
}
