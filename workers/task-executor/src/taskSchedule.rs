use rosetta_client::{BlockchainConfig, Client};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct TaskScheduler {
	config: BlockchainConfig,
	client: Client,
	callbacks: Arc<Mutex<BTreeMap<u64, Vec<Box<dyn FnMut(&Client) + Send>>>>>,
}

impl TaskScheduler {
	pub async fn new(config: BlockchainConfig, client: Client) -> Self {
		let inner: TaskScheduler = Self {
			config,
			client,
			callbacks: Default::default(),
		};
		tokio::task::spawn(inner.clone().spawn());
		inner
	}

	pub fn register_callback(&self, block_number: u64, cb: impl FnMut(&Client) + Send + 'static) {
		self.callbacks
			.lock()
			.unwrap()
			.entry(block_number)
			.or_default()
			.push(Box::new(cb));
	}

	async fn spawn(self) {
		loop {
			tokio::time::sleep(Duration::from_millis(100)).await;
			let Ok(status) = self.client.network_status(self.config.network()).await else {
				continue;
			};
			let current_block = status.current_block_identifier.index;
			let mut callbacks = self.callbacks.lock().unwrap();
			loop {
				let Some(entry) = callbacks.first_entry() else { break; };
				if *entry.key() > current_block {
					break;
				}
				for mut callback in entry.remove() {
					callback(&self.client);
				}
			}
		}
	}
}
