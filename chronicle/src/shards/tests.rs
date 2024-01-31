use super::service::{TimeWorker, TimeWorkerParams};
use crate::mock::Mock;
use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use sp_runtime::traits::IdentifyAccount;
use time_primitives::{
	sp_core, sp_runtime, AccountId, MemberStatus, PublicKey, ShardStatus, TssId, TssPublicKey,
	TssSignature, TssSigningRequest,
};
use tracing::{span, Level};

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

fn verify_tss_signature(
	public_key: TssPublicKey,
	message: &[u8],
	signature: TssSignature,
) -> Result<()> {
	let public_key = schnorr_evm::VerifyingKey::from_bytes(public_key)?;
	let signature = schnorr_evm::Signature::from_bytes(signature)?;
	public_key.verify(message, &signature)?;
	Ok(())
}

async fn sleep(seconds: u64) {
	tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await
}

#[tokio::test]
async fn tss_smoke() -> Result<()> {
	env_logger::try_init().ok();
	log_panics::init();

	let mock = Mock::new();
	let mut peers = vec![];
	let mut tss = vec![];
	for i in 1..4 {
		let (tss_tx, tss_rx) = mpsc::channel(10);
		tss.push(tss_tx);
		let worker = TimeWorker::new(TimeWorkerParams {
			network,
			tss_request: tss_rx,
			net_request,
			task_executor: task_executor.clone(),
			substrate: substrate.clone(),
		});
		tokio::task::spawn(async move {
			let span = span!(Level::INFO, "span");
			worker.run(&span).await
		});
	}

	tracing::info!("waiting for peers to connect");
	net.run_until_connected().await;
	sleep(1).await;

	let peers_account_id: Vec<(AccountId, MemberStatus)> = peers
		.iter()
		.map(|peer_id| (pubkey_from_bytes(*peer_id).into_account(), MemberStatus::Ready))
		.collect();
	let shard_id = api.create_shard(peers_account_id, 2);

	tracing::info!("waiting for shard to go online");
	while api.shard_status(shard_id) != ShardStatus::Online {
		sleep(1).await;
	}
	let public_key = mock.shard_public_key(shard_id);

	let block_number = client.chain_info().finalized_number;
	let message = [1u8; 32];
	let mut rxs = FuturesUnordered::new();
	for tss in &mut tss {
		let (tx, rx) = oneshot::channel();
		tss.send(TssSigningRequest {
			request_id: TssId(1, 1),
			shard_id: 0,
			block_number: block_number.try_into().unwrap(),
			data: message.to_vec(),
			tx,
		})
		.await?;
		rxs.push(rx);
	}
	let signature = rxs.next().await.unwrap()?;
	verify_tss_signature(public_key, &message, signature.1)?;

	Ok(())
}
