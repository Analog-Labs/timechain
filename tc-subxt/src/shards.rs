use time_primitives::{ShardId, ShardsPayload, TssPublicKey};

use crate::{timechain_runtime, SubxtClient};

impl ShardsPayload for SubxtClient {
	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Vec<TssPublicKey>,
		proof_of_knowledge: [u8; 65],
	) -> Vec<u8> {
		let commitment = commitment.iter().map(|data| data.0).collect::<Vec<_>>();
		let tx = timechain_runtime::tx()
			.shards()
			.commit(shard_id, commitment, proof_of_knowledge);
		self.make_transaction(&tx)
	}

	fn submit_online(&self, shard_id: ShardId) -> Vec<u8> {
		let tx = timechain_runtime::tx().shards().ready(shard_id);
		self.make_transaction(&tx)
	}
}
