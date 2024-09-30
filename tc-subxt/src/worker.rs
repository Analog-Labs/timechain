use crate::metadata::MetadataVariant;
use crate::metadata_scope;
use crate::{LegacyRpcMethods, OnlineClient, TxInBlock};
use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, StreamExt};
use subxt::config::DefaultExtrinsicParamsBuilder;
use subxt::tx::{Payload as TxPayload, SubmittableExtrinsic, TxStatus};
use subxt_signer::sr25519::Keypair;
use time_primitives::{
	AccountId, Commitment, Gateway, NetworkId, PeerId, ProofOfKnowledge, PublicKey, ShardId,
	TaskId, TaskResult,
};

pub enum Tx {
	// balances
	Transfer {
		account: AccountId,
		balance: u128,
	},
	// networks
	RegisterNetwork {
		network: NetworkId,
		chain_name: String,
		chain_network: String,
		gateway: Gateway,
		block_height: u64,
	},
	SetNetworkConfig {
		network: NetworkId,
		batch_size: u32,
		batch_offset: u32,
		batch_gas_limit: u128,
		shard_task_limit: u32,
	},
	// members
	RegisterMember {
		network: NetworkId,
		peer_id: PeerId,
		stake_amount: u128,
	},
	UnregisterMember,
	Heartbeat,
	// shards
	SetShardConfig {
		shard_size: u16,
		shard_threshold: u16,
	},
	SetElectable {
		accounts: Vec<AccountId>,
	},
	Commitment {
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	},
	Ready {
		shard_id: ShardId,
	},
	// tasks
	SubmitTaskResult {
		task_id: TaskId,
		result: TaskResult,
	},
}

pub struct SubxtWorker {
	client: OnlineClient,
	legacy_rpc: LegacyRpcMethods,
	metadata: MetadataVariant,
	keypair: Keypair,
	nonce: u64,
}

impl SubxtWorker {
	pub async fn new(
		legacy_rpc: LegacyRpcMethods,
		client: OnlineClient,
		metadata: MetadataVariant,
		keypair: Keypair,
	) -> Result<Self> {
		let mut me = Self {
			legacy_rpc,
			client,
			metadata,
			keypair,
			nonce: 0,
		};
		me.resync_nonce().await?;
		Ok(me)
	}

	pub fn public_key(&self) -> PublicKey {
		let public_key = self.keypair.public_key();
		PublicKey::Sr25519(unsafe { std::mem::transmute(public_key) })
	}

	pub fn account_id(&self) -> AccountId {
		let account_id: subxt::utils::AccountId32 = self.keypair.public_key().into();
		unsafe { std::mem::transmute(account_id) }
	}

	async fn create_signed_payload<Call>(&self, call: &Call) -> Vec<u8>
	where
		Call: TxPayload,
	{
		let params = DefaultExtrinsicParamsBuilder::new().nonce(self.nonce).build();
		self.client
			.tx()
			.create_signed(call, &self.keypair, params)
			.await
			.unwrap()
			.into_encoded()
	}

	async fn resync_nonce(&mut self) -> Result<()> {
		let account_id: subxt::utils::AccountId32 = self.keypair.public_key().into();
		self.nonce = self.legacy_rpc.system_account_next_index(&account_id).await?;
		Ok(())
	}

	pub async fn submit(&mut self, tx: (Tx, oneshot::Sender<TxInBlock>)) {
		let (transaction, sender) = tx;
		let tx = metadata_scope!(self.metadata, {
			match transaction {
				// balances
				Tx::Transfer { account, balance } => {
					let account: subxt::utils::AccountId32 =
						unsafe { std::mem::transmute(account) };
					let payload =
						metadata::tx().balances().transfer_allow_death(account.into(), balance);
					self.create_signed_payload(&payload).await
				},
				// networks
				Tx::RegisterNetwork {
					network,
					chain_name,
					chain_network,
					gateway,
					block_height,
				} => {
					let runtime_call = RuntimeCall::Networks(
						metadata::runtime_types::pallet_networks::pallet::Call::register_network {
							network,
							chain_name,
							chain_network,
							gateway,
							block_height,
						},
					);
					let payload = sudo(runtime_call);
					self.create_signed_payload(&payload).await
				},
				Tx::SetNetworkConfig {
					network,
					batch_size,
					batch_offset,
					batch_gas_limit,
					shard_task_limit,
				} => {
					let runtime_call = RuntimeCall::Networks(
						metadata::runtime_types::pallet_networks::pallet::Call::set_network_config {
							network,
							batch_size,
							batch_offset,
							batch_gas_limit,
							shard_task_limit,
						},
					);
					let payload = sudo(runtime_call);
					self.create_signed_payload(&payload).await
				},
				// members
				Tx::RegisterMember { network, peer_id, stake_amount } => {
					let public_key = unsafe { std::mem::transmute(self.public_key()) };
					let payload = metadata::tx().members().register_member(
						network,
						public_key,
						peer_id,
						stake_amount,
					);
					self.create_signed_payload(&payload).await
				},
				Tx::UnregisterMember => {
					let payload = metadata::tx().members().unregister_member();
					self.create_signed_payload(&payload).await
				},
				Tx::Heartbeat => {
					let payload = metadata::tx().members().send_heartbeat();
					self.create_signed_payload(&payload).await
				},
				// shards
				Tx::SetShardConfig { shard_size, shard_threshold } => {
					let runtime_call = RuntimeCall::Elections(
						metadata::runtime_types::pallet_elections::pallet::Call::set_shard_config {
							shard_size,
							shard_threshold,
						},
					);
					let payload = sudo(runtime_call);
					self.create_signed_payload(&payload).await
				},
				Tx::SetElectable { accounts } => {
					let electable = unsafe { std::mem::transmute(accounts) };
					let runtime_call = RuntimeCall::Elections(
						metadata::runtime_types::pallet_elections::pallet::Call::set_electable {
							electable,
						},
					);
					let payload = sudo(runtime_call);
					self.create_signed_payload(&payload).await
				},
				Tx::Commitment {
					shard_id,
					commitment,
					proof_of_knowledge,
				} => {
					let payload =
						metadata::tx().shards().commit(shard_id, commitment, proof_of_knowledge);
					self.create_signed_payload(&payload).await
				},
				Tx::Ready { shard_id } => {
					let payload = metadata::tx().shards().ready(shard_id);
					self.create_signed_payload(&payload).await
				},
				// tasks
				Tx::SubmitTaskResult { task_id, result } => {
					use metadata::runtime_types::time_primitives::task;
					let result: task::TaskResult = unsafe { std::mem::transmute(result) };
					let payload = metadata::tx().tasks().submit_task_result(task_id, result);
					self.create_signed_payload(&payload).await
				},
			}
		});

		let result: Result<TxInBlock> = async {
			let mut tx_progress = SubmittableExtrinsic::from_bytes(self.client.clone(), tx)
				.submit_and_watch()
				.await
				.map_err(|e| anyhow::anyhow!("Failed to Submit Tx {:?}", e))?;
			while let Some(status) = tx_progress.next().await {
				match status? {
					// In block, return.
					TxStatus::InBestBlock(s) | TxStatus::InFinalizedBlock(s) => return Ok(s),
					// Error scenarios; return the error.
					TxStatus::Error { message } => {
						anyhow::bail!("tx error: {message}");
					},
					TxStatus::Invalid { message } => {
						anyhow::bail!("tx invalid: {message}");
					},
					TxStatus::Dropped { message } => {
						anyhow::bail!("tx dropped: {message}");
					},
					// Ignore and wait for next status event:
					_ => continue,
				}
			}
			anyhow::bail!("tx subscription dropped");
		}
		.await;

		match result {
			Ok(tx_in_block) => {
				sender.send(tx_in_block).ok();
				self.nonce += 1;
			},
			Err(err) => {
				tracing::error!("Error occured while submitting transaction: {err}");
				let nonce = self.nonce;
				if let Err(err) = self.resync_nonce().await {
					tracing::error!("failed to resync nonce: {err}");
				} else {
					tracing::info!("resynced nonce from {} to {}", nonce, self.nonce);
				}
			},
		}
	}

	pub fn into_sender(mut self) -> mpsc::UnboundedSender<(Tx, oneshot::Sender<TxInBlock>)> {
		let updater = self.client.updater();
		let (tx, mut rx) = mpsc::unbounded();
		tokio::task::spawn(async move {
			tracing::info!("starting subxt worker");
			let mut update_stream =
				updater.runtime_updates().await.context("failed to start subxt worker").unwrap();
			loop {
				futures::select! {
					tx = rx.next().fuse() => {
						let Some(tx) = tx else { continue; };
						self.submit(tx).await;
					}
					update = update_stream.next().fuse() => {
						let Some(Ok(update)) = update else { continue; };
						let version = update.runtime_version().spec_version;
						match updater.apply_update(update) {
							Ok(()) => {
								tracing::info!("Upgrade to version: {} successful", version)
							},
							Err(subxt::client::UpgradeError::SameVersion) => {}
							Err(e) => {
								tracing::error!("Upgrade to version {} failed {:?}", version, e);
							},
						};
					}
				}
			}
		});
		tx
	}
}
