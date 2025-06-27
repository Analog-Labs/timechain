use crate::{Message, TableRef, Tc};
use anyhow::{Context, Result};
use csv::{Reader, StringRecord, Writer};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::{
	collections::{HashMap, VecDeque},
	fs::File,
	time::Duration,
};
use time_primitives::{Address32, BatchId, BlockHash, BlockNumber, MessageId, NetworkId};
use tokio::time::interval;

#[derive(Clone)]
struct RouteStats {
	src_addr: Address32,
	dest_addr: Address32,
	gas_limit: u64,
	msg_cost: u128,
	msg_cost_usd: f64,
	num_sent: u64,
	num_received: u64,
	processing_latency: u64,
	message_latency: u64,
	first_msg_sent: u32,
	last_msg_sent: u32,
	first_msg_completed: u32,
	last_msg_completed: u32,
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
			processing_latency: 0,
			message_latency: 0,
			first_msg_sent: 0,
			last_msg_sent: 0,
			first_msg_completed: 0,
			last_msg_completed: 0,
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
	pub processing_latency: f64,
	pub message_latency: f64,
	pub sending_throughput: f64,
	pub completion_throughput: f64,
}

#[derive(Clone, Copy)]
struct MessageStats {
	// synthetic task id not the actual task id
	task_index: u64,
	src: NetworkId,
	dest: NetworkId,
	sent_block: BlockNumber,
	batch_id: Option<BatchId>,
	received_on_timechain: Option<BlockNumber>,
	completed_on_timechain: Option<BlockNumber>,
}

impl MessageStats {
	pub fn new(task_index: u64, src: NetworkId, dest: NetworkId, sent_block: BlockNumber) -> Self {
		Self {
			task_index,
			src,
			dest,
			sent_block,
			batch_id: None,
			received_on_timechain: None,
			completed_on_timechain: None,
		}
	}
}

pub struct Benchmark {
	routes: HashMap<(NetworkId, NetworkId), RouteStats>,
	messages: HashMap<MessageId, MessageStats>,
	tc: Tc,
	payload: Vec<u8>,
	num_msgs: u64,
	latest_block: BlockNumber,
	csv_writer: Option<Writer<File>>,
	csv_path: String,
}

impl Benchmark {
	pub fn new(tc: Tc, payload: Vec<u8>, num_msgs: u64, csv_path: String) -> Self {
		Self {
			routes: Default::default(),
			messages: Default::default(),
			tc,
			payload,
			latest_block: 0,
			num_msgs,
			csv_writer: None,
			csv_path,
		}
	}

	fn init_csv_file(&mut self) -> Result<()> {
		let file = File::create(&self.csv_path)?;
		let mut writer = csv::Writer::from_writer(file);
		writer.write_record([
			"path",
			"task_index",
			"msg_id",
			"batch_id",
			"msg_sent_to_src_chain",
			"msg_received_on_timechain",
			"msg_completed_on_timechain",
		])?;

		writer.flush()?;
		self.csv_writer = Some(writer);
		Ok(())
	}

	fn write_message_to_csv(&mut self, msg_id: MessageId, msg: &MessageStats) -> Result<()> {
		if let Some(writer) = &mut self.csv_writer {
			let path = format!("{}-{}", msg.src, msg.dest);
			let msg_id_hex = hex::encode(msg_id);

			writer.write_record(&[
				path,
				msg.task_index.to_string(),
				msg_id_hex,
				msg.batch_id.map_or("".to_string(), |b| b.to_string()),
				msg.sent_block.to_string(),
				msg.received_on_timechain.map_or("".to_string(), |b| b.to_string()),
				msg.completed_on_timechain.map_or("".to_string(), |b| b.to_string()),
			])?;

			writer.flush()?;
		}
		Ok(())
	}

	async fn route_stats(
		&self,
		src: NetworkId,
		dest: NetworkId,
		block_hash: BlockHash,
	) -> Result<RouteStats> {
		let src_addr = self.tc.tester(src)?;
		let dest_addr = self.tc.tester(dest)?;
		let gas_limit = self
			.tc
			.estimate_message_gas_limit(dest, dest_addr, src, src_addr, self.payload.clone())
			.await?;
		let gas_cost = self
			.tc
			.estimate_message_cost(src, dest, self.payload.len() as u16, gas_limit, block_hash)
			.await?;
		let msg_cost = self.tc.config.network(src)?.balance_to_usd(gas_cost)?;
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
				route.msg_cost,
				self.payload.clone(),
			)
			.await?;

		Ok(message_id)
	}

	async fn update_msgs(&mut self, block: (BlockHash, BlockNumber)) -> Result<()> {
		let message_ids: Vec<MessageId> = self.messages.keys().copied().collect();
		let msg_data = self.collect_message_data(&message_ids, block.0).await?;
		self.process_message_data(msg_data, block).await?;
		Ok(())
	}

	async fn collect_message_data(
		&self,
		message_ids: &[MessageId],
		block_hash: BlockHash,
	) -> Result<Vec<(MessageId, Message)>> {
		let mut futures = FuturesUnordered::new();

		for &msg_id in message_ids {
			futures.push(async move {
				let msg = self.tc.message(msg_id, block_hash).await?;
				Ok::<_, anyhow::Error>((msg_id, msg))
			});
		}

		let mut results = Vec::new();
		while let Some(result) = futures.next().await {
			results.push(result?);
		}

		Ok(results)
	}

	async fn process_message_data(
		&mut self,
		msg_data: Vec<(MessageId, Message)>,
		block: (BlockHash, BlockNumber),
	) -> Result<()> {
		let mut newly_completed = Vec::new();

		for (msg_id, msg) in msg_data {
			if let Some(msg_stats) = self.messages.get_mut(&msg_id) {
				if msg.recv.is_some() && msg_stats.received_on_timechain.is_none() {
					msg_stats.received_on_timechain = Some(block.1);
				}

				if let Some(batch) = msg.batch {
					if msg_stats.batch_id.is_none() {
						msg_stats.batch_id = Some(batch);
					}
				}

				if msg.exec.is_some() && msg_stats.completed_on_timechain.is_none() {
					let Some(recv_block) = msg_stats.received_on_timechain else {
						continue;
					};
					msg_stats.completed_on_timechain = Some(block.1);
					newly_completed.push((msg_id, *msg_stats));
					let processing_latency = block.1 - recv_block;
					let message_latency = block.1 - msg_stats.sent_block;
					if let Some(route) = self.routes.get_mut(&(msg_stats.src, msg_stats.dest)) {
						route.processing_latency += processing_latency as u64;
						route.message_latency += message_latency as u64;
					}
				}
			}
		}

		for (msg_id, msg_stats) in newly_completed {
			self.write_message_to_csv(msg_id, &msg_stats)?;
			if let Some(route) = self.routes.get_mut(&(msg_stats.src, msg_stats.dest)) {
				route.num_received += 1;
				if route.first_msg_completed == 0 {
					route.first_msg_completed = block.1;
				}
				route.last_msg_completed = block.1;
			}
			self.messages.remove(&msg_id);
		}

		Ok(())
	}

	async fn print_stats(&self, id: Option<TableRef>) -> Result<TableRef> {
		let mut stats = Vec::with_capacity(self.routes.len());
		for ((src, dest), route) in &self.routes {
			let sending_throughput = {
				let blocks = route.last_msg_sent as i32 - route.first_msg_sent as i32;
				let total_msgs = route.num_sent;
				if blocks < 0 {
					0.0
				} else {
					total_msgs as f64 / blocks.max(1) as f64
				}
			};

			let completion_throughput = {
				let blocks = route.last_msg_completed as i32 - route.first_msg_completed as i32;
				let total_msgs = route.num_received;
				if blocks < 0 {
					0.0
				} else {
					total_msgs as f64 / blocks.max(1) as f64
				}
			};

			let processing_latency = if route.num_received > 0 {
				route.processing_latency as f64 / route.num_received as f64
			} else {
				0.0
			};

			let message_latency = if route.num_received > 0 {
				route.message_latency as f64 / route.num_received as f64
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
				processing_latency,
				message_latency,
				sending_throughput,
				completion_throughput,
			});
		}
		self.tc.print_table(id, "benchmark", stats).await
	}

	pub fn sort_csv_file(&self) -> Result<()> {
		let file = File::open(&self.csv_path)?;
		let mut reader = Reader::from_reader(file);

		let headers = reader.headers()?.clone();

		let mut records: Vec<(String, u64, StringRecord)> = Vec::new();

		for result in reader.records() {
			let record = result?;
			let path = record.get(0).unwrap_or("").to_string();
			let task_index: u64 = record.get(1).unwrap_or("0").parse().unwrap_or(0);
			records.push((path, task_index, record));
		}

		records.sort_by(|a, b| match a.0.cmp(&b.0) {
			std::cmp::Ordering::Equal => a.1.cmp(&b.1),
			other => other,
		});

		let file = File::create(&self.csv_path)?;
		let mut writer = Writer::from_writer(file);

		writer.write_record(&headers)?;

		for (_, _, record) in records {
			writer.write_record(&record)?;
		}

		writer.flush()?;
		Ok(())
	}

	pub async fn exec(&mut self) -> Result<()> {
		self.init_csv_file()?;
		let mut id = None;
		let routes: Vec<_> = self.routes.keys().copied().collect();

		for (src, dest) in routes {
			let mut task_index: u64 = 0;
			let mut messages_sent = 0;
			let mut is_first_msg_sent = false;

			let latest_block = self.tc.latest_block().await?;
			self.latest_block = latest_block.1;

			let mut block_stream = self.tc.finality_notification_stream();
			let mut unprocessed_blocks = VecDeque::new();
			let mut block_stream_initiated = false;
			let mut send_break = interval(Duration::from_millis(500));

			while messages_sent < self.num_msgs {
				tokio::select! {
					biased;
					Some(block) = block_stream.next() => {
						block_stream_initiated = true;
						self.latest_block = block.1;
						unprocessed_blocks.push_back(block);
					}
					_ = send_break.tick(), if block_stream_initiated => {
						tracing::info!("Sending msg: {} from {} to {} ", task_index + 1, src, dest);
						match self.send_single_message(src, dest).await {
							Ok(msg_id) => {
								self.messages.insert(
									msg_id,
									MessageStats::new(task_index, src, dest, self.latest_block)
								);
								task_index += 1;
								messages_sent += 1;

								if let Some(route) = self.routes.get_mut(&(src, dest)) {
									if !is_first_msg_sent {
										route.first_msg_sent = self.latest_block;
										is_first_msg_sent = true;
									}
									route.last_msg_sent = self.latest_block;
									route.num_sent += 1;
								}

								if messages_sent % 10 == 0 {
									tracing::info!("Sent {}/{} messages", messages_sent, self.num_msgs);
								}
							}
							Err(e) => {
								tracing::error!("Error sending message: {:?}", e);
							}
						}
					}
				}
			}

			tracing::info!("All {} messages sent. Starting block processing...", self.num_msgs);
			while let Some(block) = unprocessed_blocks.pop_front() {
				tracing::info!("[PROCESS] Handling block {}", block.1);
				self.update_msgs(block).await?;
				id = Some(self.print_stats(id).await?);
			}

			let mut received = self.routes.get(&(src, dest)).map_or(0, |r| r.num_received);
			while received < self.num_msgs {
				if let Some(block) = block_stream.next().await {
					tracing::info!("[PROCESS] Handling live block {}", block.1);
					self.update_msgs(block).await?;
					id = Some(self.print_stats(id).await?);
					received = self.routes.get(&(src, dest)).map_or(0, |r| r.num_received);
				}
			}

			tracing::info!(
				"Route {}-{} completed: {}/{} messages received",
				src,
				dest,
				received,
				self.num_msgs
			);
		}

		self.sort_csv_file()?;
		Ok(())
	}
}
