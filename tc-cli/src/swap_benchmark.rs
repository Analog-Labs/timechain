use crate::{BenchmarkStats, TableRef, Tc};
use anyhow::{Context, Result};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::collections::HashMap;
use time_primitives::{Address32, BlockHash, BlockNumber, MessageId, NetworkId};

#[derive(Clone, Copy)]
struct RouteStats {
	num_sent: u64,
	num_received: u64,
	sum_latency: u64,
}

impl RouteStats {
	pub fn new() -> Self {
		Self {
			num_sent: 0,
			num_received: 0,
			sum_latency: 0,
		}
	}
}

#[derive(Clone, Copy)]
struct MessageStats {
	src: NetworkId,
	dest: NetworkId,
	block: BlockNumber,
}

impl MessageStats {
	pub fn new(src: NetworkId, dest: NetworkId, block: BlockNumber) -> Self {
		Self { src, dest, block }
	}
}

pub struct SwapBenchmark {
	src: NetworkId,
	dest: NetworkId,
	// (zenswap_contract, zenswap_plugin_contract)
	src_contracts: (Address32, Address32),
	// (dest_zenswap_contract, dest_zenswap_plugin_contract)
	dest_contracts: (Address32, Address32),
	messages: HashMap<MessageId, MessageStats>,
	route: RouteStats,
	tc: Tc,
	blocks: BlockNumber,
	num_blocks: BlockNumber,
	msgs_per_block: u16,
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
		msgs_per_block: u16,
		num_blocks: BlockNumber,
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
			blocks: 0,
			num_blocks,
			msgs_per_block,
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

	async fn send_swap(&mut self, block: BlockNumber) -> Result<()> {
		let src = self.src;
		let dest = self.dest;
		for i in 0..self.msgs_per_block {
			tracing::info!("Sending swap from {} to {} of count {}", src, dest, i);
			let message_id = self
				.tc
				.send_swap(
					src,
					dest,
					self.src_contracts.0,
					self.src_contracts.1,
					self.dest_contracts.0,
					self.dest_contracts.1,
				)
				.await?;
			self.messages.insert(message_id, MessageStats::new(src, dest, block));
		}
		self.route.num_sent += self.msgs_per_block as u64;
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
				let latency = block.1 - msg.block;
				self.route.num_received += 1;
				self.route.sum_latency += latency as u64;
			}
		}
		Ok(())
	}

	async fn on_block(&mut self, block: (BlockHash, BlockNumber)) -> Result<bool> {
		let mut finished = true;
		if self.num_blocks > self.blocks {
			self.blocks += 1;
			tracing::info!("on block: {}, sending_swap", block.1);
			self.send_swap(block.1).await?;
			finished = false;
		}
		if !self.messages.is_empty() {
			self.receive_messages(block).await?;
			finished = false;
		}
		Ok(finished)
	}

	async fn print_stats(&self, id: Option<TableRef>) -> Result<TableRef> {
		let stats = BenchmarkStats {
			src: self.src,
			dest: self.dest,
			num_sent: self.route.num_sent,
			num_received: self.route.num_received,
			num_total: self.msgs_per_block as u64 * self.num_blocks as u64,
			latency: self.route.sum_latency as f64 / self.route.num_received as f64,
			throughput: self.route.num_received as f64 / self.blocks as f64,
			msg_cost: 0.0,
		};
		self.tc.print_table(id, "benchmark", vec![stats]).await
	}

	pub async fn exec(&mut self) -> Result<()> {
		let mut blocks = self.tc.finality_notification_stream();
		let mut id = None;
		loop {
			let (hash, block) = blocks.next().await.context("expected block")?;
			let finished = self.on_block((hash, block)).await?;
			id = Some(self.print_stats(id).await?);
			if finished {
				break;
			}
		}
		Ok(())
	}
}
