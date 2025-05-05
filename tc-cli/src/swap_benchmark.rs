use crate::{BenchmarkStats, TableRef, Tc};
use anyhow::{Context, Result};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::{collections::HashMap, time::Duration};
use time_primitives::{Address32, BlockHash, BlockNumber, MessageId, NetworkId};
use tokio::time::interval;

#[derive(Clone, Copy)]
struct RouteStats {
	num_sent: u64,
	num_received: u64,
	sum_latency: u64,
	first_msg_sent: BlockNumber,
	last_msg_received: BlockNumber,
}

impl RouteStats {
	pub fn new() -> Self {
		Self {
			num_sent: 0,
			num_received: 0,
			sum_latency: 0,
			first_msg_sent: 1,
			last_msg_received: 1,
		}
	}
}

#[derive(Clone, Copy)]
struct MessageStats {
	sent_block: BlockNumber,
}

impl MessageStats {
	pub fn new(sent_block: BlockNumber) -> Self {
		Self { sent_block }
	}
}

pub struct SwapBenchmark {
	tc: Tc,
	src: NetworkId,
	dest: NetworkId,
	// (zenswap_contract, zenswap_plugin_contract)
	src_contracts: (Address32, Address32),
	// (dest_zenswap_contract, dest_zenswap_plugin_contract)
	dest_contracts: (Address32, Address32),
	messages: HashMap<MessageId, MessageStats>,
	current_block: BlockNumber,
	route: RouteStats,
	// blocks: BlockNumber,
	// num_blocks: BlockNumber,
	total_msgs: u64,
}

impl SwapBenchmark {
	pub fn new(
		tc: Tc,
		src: NetworkId,
		dest: NetworkId,
		zenswap: Address32,
		zenswap_plug: Address32,
		dest_zenswap: Address32,
		dest_zenswap_plug: Address32,
		total_msgs: u64,
	) -> Self {
		let route = RouteStats::new();
		Self {
			tc,
			src,
			dest,
			src_contracts: (zenswap, zenswap_plug),
			dest_contracts: (dest_zenswap, dest_zenswap_plug),
			messages: Default::default(),
			route,
			current_block: 0,
			total_msgs,
		}
	}

	pub async fn wait_for_sync(&mut self) -> Result<()> {
		let mut sync = FuturesUnordered::new();
		for network in self.tc.iter() {
			if network == self.src || network == self.dest {
				sync.push(self.tc.wait_for_sync(network));
			}
		}
		while let Some(result) = sync.next().await {
			result?;
		}
		Ok(())
	}

	async fn receive_messages(&mut self, block: (BlockHash, BlockNumber)) -> Result<()> {
		let mut messages = FuturesUnordered::new();
		for message_id in self.messages.keys().copied() {
			let fut = self.tc.is_message_executed(message_id, block.0);
			messages.push(async move {
				let is_executed = fut.await?;
				Ok::<_, anyhow::Error>((message_id, is_executed))
			});
		}
		while let Some(result) = messages.next().await {
			let (message_id, is_executed) = result?;
			if is_executed {
				let Some(msg) = self.messages.remove(&message_id) else {
					continue;
				};
				let latency = block.1 - msg.sent_block;
				self.route.num_received += 1;
				self.route.sum_latency += latency as u64;
				self.route.last_msg_received = self.current_block;
			}
		}
		Ok(())
	}

	async fn print_stats(&self, id: Option<TableRef>) -> Result<TableRef> {
		let total_blocks = if self.route.first_msg_sent <= self.route.last_msg_received {
			(self.route.last_msg_received - self.route.first_msg_sent + 1) as f64
		} else {
			0.0
		};
		let stats = BenchmarkStats {
			src: self.src,
			dest: self.dest,
			num_sent: self.route.num_sent,
			num_received: self.route.num_received,
			num_total: self.total_msgs,
			latency: self.route.sum_latency as f64 / self.route.num_received as f64,
			throughput: self.route.num_received as f64 / total_blocks as f64,
			msg_cost: 0.0,
		};
		self.tc.print_table(id, "benchmark", vec![stats]).await
	}

	pub async fn exec(&mut self) -> Result<()> {
		let mut blocks = self.tc.finality_notification_stream();
		let mut id = None;
		let mut send_interval = interval(Duration::from_secs(2));
		let latest_block = self.tc.latest_block().await?;
		self.current_block = latest_block.1;
		loop {
			tokio::select! {
				block = blocks.next() => {
					let (hash, number) = block.context("expected block")?;
					self.current_block = number;
					self.receive_messages((hash, number)).await?;
					id = Some(self.print_stats(id).await?);
					if self.route.num_received >= self.total_msgs {
						tracing::info!("Benchmark completed");
						break;
					}
				}
				_ = send_interval.tick(), if self.route.num_sent < self.total_msgs => {
					if self.route.first_msg_sent == 1 {
						self.route.first_msg_sent = self.current_block;
					}
					let message_id = self.tc.send_swap(
						self.src,
						self.dest,
						self.src_contracts.0,
						self.src_contracts.1,
						self.dest_contracts.0,
						self.dest_contracts.1,
						// only needed to get the network_chain so old block is fine
						latest_block.0
					).await?;

					self.messages.insert(
						message_id,
						MessageStats::new(self.current_block)
					);
					self.route.num_sent += 1;
				}
			}
		}
		Ok(())
	}
}
