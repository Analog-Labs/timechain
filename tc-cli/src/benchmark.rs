use crate::{TableRef, Tc};
use anyhow::{Context, Result};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::{collections::HashMap, time::Duration};
use time_primitives::{Address32, BlockHash, BlockNumber, MessageId, NetworkId};
use tokio::time::interval;

#[derive(Clone, Copy)]
struct RouteStats {
	src_addr: Address32,
	dest_addr: Address32,
	gas_limit: u64,
	msg_cost: u128,
	msg_cost_usd: f64,
	num_sent: u64,
	num_received: u64,
	sum_latency: u64,
	first_msg_sent: BlockNumber,
	last_msg_received: BlockNumber,
}

impl RouteStats {
	pub fn new(
		src_addr: Address32,
		dest_addr: Address32,
		gas_limit: u64,
		msg_cost: u128,
		msg_cost_usd: f64,
	) -> Self {
		Self {
			src_addr,
			dest_addr,
			gas_limit,
			msg_cost,
			msg_cost_usd,
			num_sent: 0,
			num_received: 0,
			sum_latency: 0,
			first_msg_sent: BlockNumber::MAX,
			last_msg_received: 0,
		}
	}
}

#[derive(Clone, Copy)]
pub struct BenchmarkStats {
	pub src: NetworkId,
	pub dest: NetworkId,
	pub msg_cost_usd: f64,
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
	sent_block: BlockNumber,
}

impl MessageStats {
	pub fn new(src: NetworkId, dest: NetworkId, sent_block: BlockNumber) -> Self {
		Self { src, dest, sent_block }
	}
}

pub struct Benchmark {
	routes: HashMap<(NetworkId, NetworkId), RouteStats>,
	messages: HashMap<MessageId, MessageStats>,
	tc: Tc,
	payload: Vec<u8>,
	num_msgs: u64,
	latest_block: BlockNumber,
	// msgs_per_block: u16,
}

impl Benchmark {
	pub fn new(tc: Tc, payload: Vec<u8>, num_msgs: u64) -> Self {
		Self {
			routes: Default::default(),
			messages: Default::default(),
			tc,
			payload,
			latest_block: 0,
			num_msgs,
		}
	}

	async fn route_stats(
		&self,
		src: NetworkId,
		dest: NetworkId,
		block_hash: BlockHash,
	) -> Result<RouteStats> {
		let src_addr = self.tc.tester(src)?.0;
		let dest_addr = self.tc.tester(dest)?.0;
		let gas_limit = self
			.tc
			.estimate_message_gas_limit(dest, dest_addr, src, src_addr, self.payload.clone())
			.await?;
		let gas_cost = self
			.tc
			.estimate_message_cost(src, dest, self.payload.len() as u16, gas_limit, block_hash)
			.await?;
		let msg_cost = self.tc.config.balance_to_usd(src, gas_cost)?;
		Ok(RouteStats::new(src_addr, dest_addr, gas_limit, gas_cost, msg_cost))
	}

	pub async fn add_routes(&mut self, block_hash: BlockHash) -> Result<()> {
		let routes = FuturesUnordered::new();
		for src in self.tc.iter() {
			for dest in self.tc.iter() {
				if src != dest {
					let fut = self.route_stats(src, dest, block_hash);
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
		for network in self.tc.iter() {
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
					route.msg_cost,
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

	async fn send_single_message(&self, src: NetworkId, dest: NetworkId) -> Result<MessageId> {
		let route = self.routes.get(&(src, dest)).context("Route not found")?;
		let message_id = self
			.tc
			.send_message(
				src,
				route.src_addr,
				dest,
				route.dest_addr,
				route.gas_limit,
				route.gas_cost,
				self.payload.clone(),
			)
			.await?;

		Ok(message_id)
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
				let Some(route) = self.routes.get_mut(&(msg.src, msg.dest)) else {
					continue;
				};
				let latency = block.1 - msg.sent_block;
				route.num_received += 1;
				route.sum_latency += latency as u64;

				if msg.sent_block < route.first_msg_sent {
					route.first_msg_sent = msg.sent_block;
				}
				if block.1 > route.last_msg_received {
					route.last_msg_received = block.1;
				}
			}
		}
		Ok(())
	}

	async fn print_stats(&self, id: Option<TableRef>) -> Result<TableRef> {
		let mut stats = Vec::with_capacity(self.routes.len());
		for ((src, dest), route) in &self.routes {
			let total_blocks = if route.first_msg_sent <= route.last_msg_received {
				(route.last_msg_received - route.first_msg_sent + 1) as f64
			} else {
				0.0
			};

			let latency = if route.num_received > 0 {
				route.sum_latency as f64 / route.num_received as f64
			} else {
				0.0
			};

			stats.push(BenchmarkStats {
				src: *src,
				dest: *dest,
				msg_cost_usd: route.msg_cost_usd,
				num_sent: route.num_sent,
				num_received: route.num_received,
				num_total: self.num_msgs,
				latency,
				throughput: route.num_received as f64 / total_blocks as f64,
			});
		}
		self.tc.print_table(id, "benchmark", stats).await
	}

	pub async fn exec(&mut self) -> Result<()> {
		let mut id = None;
		let routes: Vec<_> = self.routes.keys().copied().collect();

		for (src, dest) in routes {
			let mut messages_sent = 0;

			let latest_block = self.tc.latest_block().await?;
			self.latest_block = latest_block.1;

			let mut send_interval = interval(Duration::from_secs(2));
			let mut block_stream = self.tc.finality_notification_stream();

			loop {
				tokio::select! {
					Some(block) = block_stream.next() => {
						self.latest_block = block.1;
						self.receive_messages(block).await?;
						id = Some(self.print_stats(id).await?);

						if let Some(route) = self.routes.get(&(src, dest)) {
							if route.num_received >= self.num_msgs {
								break;
							}
						}
					}

					_ = send_interval.tick(), if messages_sent < self.num_msgs => {
						match self.send_single_message(src, dest).await {
							Ok(msg_id) => {
								self.messages.insert(
									msg_id,
									MessageStats::new(src, dest, self.latest_block)
								);
								messages_sent += 1;

								if let Some(route) = self.routes.get_mut(&(src, dest)) {
									route.num_sent += 1;
									if route.first_msg_sent == BlockNumber::MAX {
										route.first_msg_sent = self.latest_block;
									}
								}
							}
							Err(e) => {
								tracing::error!("Error sending message: {:?}", e);
							}
						}
					}
				}

				let received = self.routes.get(&(src, dest)).map_or(0, |r| r.num_received);
				if messages_sent >= self.num_msgs && received >= self.num_msgs {
					break;
				}
			}
		}

		Ok(())
	}
}
