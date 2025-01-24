use crate::metadata::{self, runtime_types, RuntimeCall};
use crate::{ExtrinsicEvents, LegacyRpcMethods, OnlineClient};

use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, StreamExt};
use subxt::backend::rpc::RpcClient;
use subxt::config::DefaultExtrinsicParamsBuilder;
use subxt::tx::{Payload as TxPayload, SubmittableExtrinsic};
use subxt::utils::H256;
use subxt_signer::sr25519::Keypair;
use time_primitives::{
	traits::IdentifyAccount, AccountId, Commitment, GmpEvents, Network, NetworkConfig, NetworkId,
	PeerId, ProofOfKnowledge, PublicKey, ShardId, TaskId, TaskResult,
};

const MORTALITY: u8 = 15;

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
	ForceShardOffline {
		shard_id: ShardId,
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
	keypair: Keypair,
	nonce: u64,
}

impl SubxtWorker {
	pub async fn new(rpc: RpcClient, client: OnlineClient, keypair: Keypair) -> Result<Self> {
		let mut me = Self { rpc, client, keypair, nonce: 0 };
		me.resync_nonce().await?;
		Ok(me)
	}

	pub fn public_key(&self) -> PublicKey {
		PublicKey::Sr25519(self.keypair.public_key().as_ref().try_into().unwrap())
	}

	pub fn account_id(&self) -> AccountId {
		self.public_key().into_account()
	}

	async fn create_signed_payload<Call>(&self, call: &Call) -> (u64, Vec<u8>)
	where
		Call: TxPayload,
	{
		let latest_block = self.client.blocks().at_latest().await.unwrap();
		let params = DefaultExtrinsicParamsBuilder::new()
			.nonce(self.nonce)
			.mortal(latest_block.header(), MORTALITY.into())
			.build();
		let tx = self
			.client
			.tx()
			.create_signed(call, &self.keypair, params)
			.await
			.unwrap()
			.into_encoded();
		let block_number: u64 = latest_block.number().into();
		(block_number + MORTALITY as u64, tx)
	}

	async fn resync_nonce(&mut self) -> Result<()> {
		let account_id: subxt::utils::AccountId32 = self.keypair.public_key().into();
		let rpc = LegacyRpcMethods::new(self.rpc.clone());
		self.nonce = rpc.system_account_next_index(&account_id).await?;
		Ok(())
	}

	pub async fn submit(&mut self, tx: (Tx, oneshot::Sender<ExtrinsicEvents>)) {
		let (transaction, sender) = tx;
		let (era, tx) = match transaction {
			// system
			Tx::SetCode { code } => {
				let runtime_call =
					RuntimeCall::System(runtime_types::frame_system::pallet::Call::set_code {
						code,
					});
				let payload = metadata::sudo(runtime_call);
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
					runtime_types::pallet_networks::pallet::Call::register_network { network },
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload).await
			},
			Tx::ForceShardOffline { shard_id } => {
				let runtime_call = RuntimeCall::Shards(
					metadata::runtime_types::pallet_shards::pallet::Call::force_shard_offline {
						shard_id,
					},
				);
				let payload = metadata::sudo(runtime_call);
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
				let payload = metadata::sudo(runtime_call);
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
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload).await
			},
			Tx::RemoveTask { task_id } => {
				let runtime_call = RuntimeCall::Tasks(
					metadata::runtime_types::pallet_tasks::pallet::Call::remove_task {
						task: task_id,
					},
				);
				let payload = metadata::sudo(runtime_call);
				self.create_signed_payload(&payload).await
			},
		};

		let result: Result<ExtrinsicEvents> = async {
			let submitted_extrinsic_hash =
				SubmittableExtrinsic::from_bytes(self.client.clone(), tx)
					.submit()
					.await
					.map_err(|e| anyhow::anyhow!("Failed to Submit Tx {:?}", e))?;
			tracing::info!("extrinsic_hash :{:?}", submitted_extrinsic_hash);

			loop {
				match self.watch_tx(submitted_extrinsic_hash, era).await {
					Ok(Ok(events)) => return Ok(events),
					Ok(Err(e)) => {
						tracing::error!(
							"Error occured while listening tx status: {e}, Retrying..."
						);
						tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
						continue;
					},
					Err(e) => anyhow::bail!("Error occured with tokio handler: {:?}", e),
				}
			}
		}
		.await;

		match result {
			Ok(events) => {
				sender.send(events).ok();
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

	async fn watch_tx(
		&self,
		submitted_extrinsic_hash: H256,
		era: u64,
	) -> Result<Result<ExtrinsicEvents>> {
		let mut finalized_block_stream = self.client.blocks().subscribe_best().await?;
		let result = tokio::task::spawn(async move {
			'outer: loop {
				let Some(block) = finalized_block_stream.next().await else {
					anyhow::bail!("Stream was closed")
				};
				let block: crate::Block = block?;
				for extrinsic in block.extrinsics().await?.iter() {
					let extrinsic_hash = extrinsic.hash();
					if submitted_extrinsic_hash == extrinsic_hash {
						let extrinsic_events = extrinsic.events().await?;
						break 'outer Ok(extrinsic_events);
					}
				}
				let current_block: u64 = block.number().into();
				if current_block > era {
					anyhow::bail!("Retry")
				}
			}
		})
		.await;
		result.map_err(|e| anyhow::anyhow!(e))
	}

	pub fn into_sender(mut self) -> mpsc::UnboundedSender<(Tx, oneshot::Sender<ExtrinsicEvents>)> {
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
