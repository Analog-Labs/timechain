use crate::{TableRef, Tc};
use anyhow::{Context, Result};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::collections::HashMap;
use time_primitives::{Address, BlockNumber, MessageId, NetworkId};

#[derive(Clone, Copy)]
struct RouteStats {
	src_addr: Address,
	dest_addr: Address,
	gas_limit: u128,
	gas_cost: u128,
	msg_cost: f64,
	num_sent: u64,
	num_received: u64,
	sum_latency: u64,
}

impl RouteStats {
	pub fn new(
		src_addr: Address,
		dest_addr: Address,
		gas_limit: u128,
		gas_cost: u128,
		msg_cost: f64,
	) -> Self {
		Self {
			src_addr,
			dest_addr,
			gas_limit,
			gas_cost,
			msg_cost,
			num_sent: 0,
			num_received: 0,
			sum_latency: 0,
		}
	}
}

#[derive(Clone, Copy)]
pub struct BenchmarkStats {
	pub src: NetworkId,
	pub dest: NetworkId,
	pub msg_cost: f64,
	pub num_sent: u64,
	pub num_received: u64,
	pub num_total: u64,
	pub latency: f64,
	pub throughput: f64,
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

pub struct Benchmark {
	routes: HashMap<(NetworkId, NetworkId), RouteStats>,
	messages: HashMap<MessageId, MessageStats>,
	tc: Tc,
	testers: HashMap<NetworkId, (Address, u64)>,
	payload: Vec<u8>,
	blocks: BlockNumber,
	num_blocks: BlockNumber,
	msgs_per_block: u16,
}

impl Benchmark {
	pub fn new(
		tc: Tc,
		testers: HashMap<NetworkId, (Address, u64)>,
		payload: Vec<u8>,
		msgs_per_block: u16,
		num_blocks: BlockNumber,
	) -> Self {
		Self {
			routes: Default::default(),
			messages: Default::default(),
			tc,
			testers,
			payload,
			blocks: 0,
			num_blocks,
			msgs_per_block,
		}
	}

	async fn route_stats(&self, src: NetworkId, dest: NetworkId) -> Result<RouteStats> {
		let src_addr = self.testers.get(&src).context("missing tester")?.0;
		let dest_addr = self.testers.get(&dest).context("missing tester")?.0;
		let gas_limit = self
			.tc
			.estimate_message_gas_limit(dest, dest_addr, src, src_addr, self.payload.clone())
			.await?;
		let gas_cost = self
			.tc
			.estimate_message_cost(src, dest, gas_limit, self.payload.clone())
			.await?;
		let msg_cost = self.tc.balance_to_usd(src, gas_cost)?;
		Ok(RouteStats::new(src_addr, dest_addr, gas_limit, gas_cost, msg_cost))
	}

	pub async fn add_routes(&mut self) -> Result<()> {
		let routes = FuturesUnordered::new();
		for src in self.testers.keys().copied() {
			for dest in self.testers.keys().copied() {
				if src != dest {
					let fut = self.route_stats(src, dest);
					routes.push(async move {
						let route = fut.await?;
						Ok::<_, anyhow::Error>((src, dest, route))
					});
				}
			}
		}
		for result in routes.collect::<Vec<_>>().await {
			let (src, dest, route) = result?;
			self.routes.insert((src, dest), route);
		}
		Ok(())
	}

	pub async fn wait_for_sync(&mut self) -> Result<()> {
		let mut sync = FuturesUnordered::new();
		for network in self.testers.keys().copied() {
			sync.push(self.tc.wait_for_sync(network));
		}
		while let Some(result) = sync.next().await {
			result?;
		}
		Ok(())
	}

	async fn send_messages(&mut self, block: BlockNumber) -> Result<()> {
		let mut messages = FuturesUnordered::new();
		for ((src, dest), route) in &mut self.routes {
			for _ in 0..self.msgs_per_block {
				let fut = self.tc.send_message(
					*src,
					route.src_addr,
					*dest,
					route.dest_addr,
					route.gas_limit,
					route.gas_cost,
					self.payload.clone(),
				);
				messages.push(async move {
					let message_id = fut.await?;
					Ok::<_, anyhow::Error>((*src, *dest, message_id))
				});
			}
			route.num_sent += self.msgs_per_block as u64;
		}
		while let Some(result) = messages.next().await {
			let (src, dest, message_id) = result?;
			self.messages.insert(message_id, MessageStats::new(src, dest, block));
		}
		Ok(())
	}

	async fn receive_messages(&mut self, block: BlockNumber) -> Result<()> {
		let mut messages = FuturesUnordered::new();
		for message_id in self.messages.keys().copied() {
			let fut = self.tc.is_message_executed(message_id);
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
				let Some(route) = self.routes.get_mut(&(msg.src, msg.dest)) else {
					continue;
				};
				let latency = block - msg.block;
				route.num_received += 1;
				route.sum_latency += latency as u64;
			}
		}
		Ok(())
	}

	async fn on_block(&mut self, block: BlockNumber) -> Result<bool> {
		let mut finished = true;
		if self.num_blocks > self.blocks {
			self.blocks += 1;
			self.send_messages(block).await?;
			finished = false;
		}
		if !self.messages.is_empty() {
			self.receive_messages(block).await?;
			finished = false;
		}
		Ok(finished)
	}

	async fn print_stats(&self, id: Option<TableRef>) -> Result<TableRef> {
		let mut stats = Vec::with_capacity(self.routes.len());
		for ((src, dest), route) in &self.routes {
			stats.push(BenchmarkStats {
				src: *src,
				dest: *dest,
				msg_cost: route.msg_cost,
				num_sent: route.num_sent,
				num_received: route.num_received,
				num_total: self.msgs_per_block as u64 * self.num_blocks as u64,
				latency: route.sum_latency as f64 / route.num_received as f64,
				throughput: route.num_received as f64 / self.blocks as f64,
			});
		}
		self.tc.print_table(id, "benchmark", stats).await
	}

	pub async fn exec(&mut self) -> Result<()> {
		let mut blocks = self.tc.finality_notification_stream();
		let mut id = None;
		loop {
			let (_, block) = blocks.next().await.context("expected block")?;
			let finished = self.on_block(block).await?;
			id = Some(self.print_stats(id).await?);
			if finished {
				break;
			}
		}
		Ok(())
	}
}
