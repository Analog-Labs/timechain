use crate::metadata::MetadataVariant;
use crate::metadata_scope;
use crate::{LegacyRpcMethods, OnlineClient, TxInBlock};
use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, StreamExt};
use subxt::backend::rpc::RpcClient;
use subxt::config::DefaultExtrinsicParamsBuilder;
use subxt::tx::{Payload as TxPayload, SubmittableExtrinsic, TxStatus};
use subxt_signer::sr25519::Keypair;
use time_primitives::{
	traits::IdentifyAccount, AccountId, Commitment, GmpEvents, Network, NetworkConfig, NetworkId,
	PeerId, ProofOfKnowledge, PublicKey, ShardId, TaskId, TaskResult,
};

pub enum Tx {
	// system
	SetCode {
		code: Vec<u8>,
	},
	// balances
	Transfer {
		account: AccountId,
		balance: u128,
	},
	// networks
	RegisterNetwork {
		network: Network,
	},
	SetNetworkConfig {
		network: NetworkId,
		config: NetworkConfig,
	},
	// members
	RegisterMember {
		network: NetworkId,
		public_key: PublicKey,
		peer_id: PeerId,
		stake_amount: u128,
	},
	UnregisterMember {
		member: AccountId,
	},
	Heartbeat,
	// shards
	SetShardConfig {
		shard_size: u16,
		shard_threshold: u16,
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
	SubmitGmpEvents {
		network: NetworkId,
		gmp_events: GmpEvents,
	},
	RemoveTask {
		task_id: TaskId,
	},
}

pub struct SubxtWorker {
	rpc: RpcClient,
	client: OnlineClient,
	metadata: MetadataVariant,
	keypair: Keypair,
	nonce: u64,
}

impl SubxtWorker {
	pub async fn new(
		rpc: RpcClient,
		client: OnlineClient,
		metadata: MetadataVariant,
		keypair: Keypair,
	) -> Result<Self> {
		let mut me = Self {
			rpc,
			client,
			metadata,
			keypair,
			nonce: 0,
		};
		me.resync_nonce().await?;
		Ok(me)
	}

	pub fn public_key(&self) -> PublicKey {
		PublicKey::Sr25519(self.keypair.public_key().as_ref().try_into().unwrap())
	}

	pub fn account_id(&self) -> AccountId {
		self.public_key().into_account()
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
		let rpc = LegacyRpcMethods::new(self.rpc.clone());
		self.nonce = rpc.system_account_next_index(&account_id).await?;
		Ok(())
	}

	pub async fn submit(&mut self, tx: (Tx, oneshot::Sender<TxInBlock>)) {
		let (transaction, sender) = tx;
		let tx = metadata_scope!(self.metadata, {
			match transaction {
				// system
				Tx::SetCode { code } => {
					let runtime_call = RuntimeCall::System(
						metadata::runtime_types::frame_system::pallet::Call::set_code { code },
					);
					let payload = sudo(runtime_call);
					self.create_signed_payload(&payload).await
				},
				// balances
				Tx::Transfer { account, balance } => {
					let account = subxt::utils::Static(account);
					let payload =
						metadata::tx().balances().transfer_allow_death(account.into(), balance);
					self.create_signed_payload(&payload).await
				},
				// networks
				Tx::RegisterNetwork { network } => {
					let network = subxt::utils::Static(network);
					let runtime_call = RuntimeCall::Networks(
						metadata::runtime_types::pallet_networks::pallet::Call::register_network {
							network,
						},
					);
					let payload = sudo(runtime_call);
					self.create_signed_payload(&payload).await
				},
				Tx::SetNetworkConfig { network, config } => {
					let config = subxt::utils::Static(config);
					let runtime_call = RuntimeCall::Networks(
						metadata::runtime_types::pallet_networks::pallet::Call::set_network_config {
							network,
							config,
						},
					);
					let payload = sudo(runtime_call);
					self.create_signed_payload(&payload).await
				},
				// members
				Tx::RegisterMember {
					network,
					public_key,
					peer_id,
					stake_amount,
				} => {
					let public_key = subxt::utils::Static(public_key);
					let payload = metadata::tx().members().register_member(
						network,
						public_key,
						peer_id,
						stake_amount,
					);
					self.create_signed_payload(&payload).await
				},
				Tx::UnregisterMember { member } => {
					let member = subxt::utils::Static(member);
					let payload = metadata::tx().members().unregister_member(member);
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
				Tx::Commitment {
					shard_id,
					commitment,
					proof_of_knowledge,
				} => {
					let commitment = subxt::utils::Static(commitment);
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
					let result = subxt::utils::Static(result);
					let payload = metadata::tx().tasks().submit_task_result(task_id, result);
					self.create_signed_payload(&payload).await
				},
				Tx::SubmitGmpEvents { network, gmp_events } => {
					let runtime_call = RuntimeCall::Tasks(
						metadata::runtime_types::pallet_tasks::pallet::Call::submit_gmp_events {
							network,
							events: subxt::utils::Static(gmp_events),
						},
					);
					let payload = sudo(runtime_call);
					self.create_signed_payload(&payload).await
				},
				Tx::RemoveTask { task_id } => {
					let runtime_call = RuntimeCall::Tasks(
						metadata::runtime_types::pallet_tasks::pallet::Call::remove_task {
							task: task_id,
						},
					);
					let payload = sudo(runtime_call);
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
