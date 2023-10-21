use crate::{timechain_runtime, SubxtClient};
use time_primitives::{Network, PeerId, PublicKey};
use timechain_runtime::runtime_types::sp_runtime::MultiSigner as MetadataMultiSigner;
use timechain_runtime::runtime_types::time_primitives::shard;

impl SubxtClient {
	pub fn register_member(
		&self,
		network: Network,
		public_key: PublicKey,
		peer_id: PeerId,
	) -> Vec<u8> {
		let network: shard::Network = unsafe { std::mem::transmute(network) };
		let public_key: MetadataMultiSigner = unsafe { std::mem::transmute(public_key) };
		let tx = timechain_runtime::tx().members().register_member(network, public_key, peer_id);
		self.make_transaction(&tx)
	}

	pub fn submit_heartbeat(&self) -> Vec<u8> {
		let tx = timechain_runtime::tx().members().send_heartbeat();
		self.make_transaction(&tx)
	}
}
