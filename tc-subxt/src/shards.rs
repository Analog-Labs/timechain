use time_primitives::{PublicKey, ShardId};

use crate::{timechain_runtime, SubxtClient, TcSubxtError};

impl SubxtClient {
	pub fn submit_commitment(
		&mut self,
		shard_id: ShardId,
		_: PublicKey,
		commitment: Vec<[u8; 33]>,
		proof_of_knowledge: [u8; 65],
	) -> Result<Vec<u8>, TcSubxtError> {
		let tx = timechain_runtime::tx()
			.shards()
			.commit(shard_id, commitment, proof_of_knowledge);
		self.make_transaction(&tx)
	}
	pub fn submit_ready(
		&mut self,
		shard_id: ShardId,
		_: PublicKey,
	) -> Result<Vec<u8>, TcSubxtError> {
		let tx = timechain_runtime::tx().shards().ready(shard_id);
		self.make_transaction(&tx)
	}
}
