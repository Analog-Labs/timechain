use crate::worker::Tx;
use crate::SubxtClient;
use anyhow::Result;
use futures::channel::oneshot;
use time_primitives::Balance;

impl SubxtClient {
	pub async fn withdraw(&self, amount: Balance) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::Withdraw { amount }, tx))?;
		rx.await?.wait_for_success().await?;
		Ok(())
	}
}
