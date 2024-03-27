use crate::{Call, Config, Pallet};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use pallet_shards::{ShardCommitment, ShardState};
use scale_info::prelude::vec;
use schnorr_evm::SigningKey;
use time_primitives::{
	AccountId, Function, GmpParams, Message, Msg, NetworkId, Payload, ShardId, ShardStatus,
	ShardsInterface, TaskDescriptorParams, TaskId, TaskResult, TasksInterface,
};

const ETHEREUM: NetworkId = 1;

pub struct MockTssSigner {
	signing_key: SigningKey,
}

impl MockTssSigner {
	pub fn new() -> Self {
		Self {
			//random key bytes
			signing_key: SigningKey::from_bytes([
				62, 78, 161, 128, 140, 236, 177, 67, 143, 75, 171, 207, 104, 60, 36, 95, 104, 71,
				17, 91, 237, 184, 132, 165, 52, 240, 194, 4, 138, 196, 89, 176,
			])
			.unwrap(),
		}
	}

	pub fn public_key(&self) -> [u8; 33] {
		self.signing_key.public().to_bytes().unwrap()
	}

	pub fn sign(&self, data: &[u8]) -> schnorr_evm::Signature {
		self.signing_key.sign(data)
	}
}

fn mock_submit_sig(function: Function) -> [u8; 64] {
	let tss_public_key = MockTssSigner::new().public_key();
	let gmp_params = GmpParams {
		network_id: ETHEREUM,
		tss_public_key,
		gateway_contract: [0u8; 20].into(),
	};
	let payload: Vec<u8> = match function {
		Function::RegisterShard { .. } => {
			let tss_pubkey = MockTssSigner::new().public_key();
			Message::update_keys([], [tss_pubkey]).to_eip712_bytes(&gmp_params).into()
		},
		Function::UnregisterShard { .. } => {
			let tss_pubkey = MockTssSigner::new().public_key();
			Message::update_keys([tss_pubkey], []).to_eip712_bytes(&gmp_params).into()
		},
		Function::SendMessage { msg } => Message::gmp(msg).to_eip712_bytes(&gmp_params).into(),
		_ => Default::default(),
	};
	MockTssSigner::new().sign(&payload).to_bytes()
}

fn mock_result_ok(shard_id: ShardId, task_id: TaskId) -> ([u8; 33], TaskResult) {
	// these values are taken after running a valid instance of submitting result
	let hash = [
		11, 210, 118, 190, 192, 58, 251, 12, 81, 99, 159, 107, 191, 242, 96, 233, 203, 127, 91, 0,
		219, 14, 241, 19, 45, 124, 246, 145, 176, 169, 138, 11,
	];
	let payload = Payload::Hashed(hash);
	let signer = MockTssSigner::new();
	let signature = signer.sign(&payload.bytes(task_id)).to_bytes();
	(signer.public_key(), TaskResult { shard_id, payload, signature })
}

benchmarks! {
	where_clause {  where T: pallet_shards::Config }
	create_task {
		let b in 1..10000;
		let input = vec![0u8; b as usize];
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::EvmViewCall {
				address: Default::default(),
				input,
			},
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		};
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let caller = whitelisted_caller();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(10_000),
		);
	}: _(RawOrigin::Signed(whitelisted_caller()), descriptor) verify {}

	submit_result {
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::EvmViewCall {
				address: Default::default(),
				input: Default::default(),
			},
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		};
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let caller = whitelisted_caller();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(10_000),
		);
		Pallet::<T>::create_task(RawOrigin::Signed(caller.clone()).into(), descriptor)?;
		let (pub_key, result) = mock_result_ok(0, 0);
		ShardCommitment::<T>::insert(0, vec![pub_key]);
	}: _(RawOrigin::Signed(caller), 0, result) verify {}

	submit_hash {
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			start: 0,
			function: Function::EvmCall {
				address: Default::default(),
				input: Default::default(),
				amount: 0,
				gas_limit: None,
			},
			funds: 100,
			shard_size: 3,
		};
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let caller: AccountId= [0u8; 32].into();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(10_000),
		);
		Pallet::<T>::create_task(RawOrigin::Signed(caller.clone()).into(), descriptor)?;
	}: _(RawOrigin::Signed(caller), 0, [0u8; 32]) verify {}

	submit_signature {
		let function = Function::SendMessage { msg: Msg::default() };
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			start: 0,
			function: function.clone(),
			funds: 100,
			shard_size: 3,
		};
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let caller: AccountId= [0u8; 32].into();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(10_000),
		);
		Pallet::<T>::create_task(RawOrigin::Signed(caller.clone()).into(), descriptor)?;
		Pallet::<T>::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0)?;
		ShardCommitment::<T>::insert(0, vec![MockTssSigner::new().public_key()]);
		let signature = mock_submit_sig(function);
	}: _(RawOrigin::Signed(caller), 0, signature) verify {}

	register_gateway {
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
	}: _(RawOrigin::Root, 0, [0u8; 20], 20) verify {}

	set_read_task_reward {
	}: _(RawOrigin::Root, 0, 20) verify {}

	set_write_task_reward {
	}: _(RawOrigin::Root, 0, 20) verify {}

	set_send_message_task_reward {
	}: _(RawOrigin::Root, 0, 20) verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
