use time_primitives::ShardId;

use crate::{timechain_runtime, SubxtClient};

impl SubxtClient {
	pub fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Vec<[u8; 33]>,
		proof_of_knowledge: [u8; 65],
	) -> Vec<u8> {
		let tx = timechain_runtime::tx()
			.shards()
			.commit(shard_id, commitment, proof_of_knowledge);
		self.make_transaction(&tx)
	}

	pub fn submit_ready(&self, shard_id: ShardId) -> Vec<u8> {
		let tx = timechain_runtime::tx().shards().ready(shard_id);
		self.make_transaction(&tx)
	}
}
