use anyhow::Result;
use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tide::{Body, Request, Response, StatusCode};
use time_primitives::admin::Config;
use time_primitives::ShardId;
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum AdminMsg {
	SetConfig(Config),
	JoinedShard(ShardId),
	TargetBlockReceived,
	FailedTasks(u64, u64),
}

#[derive(Clone)]
pub enum ChronicleError {
	MaybeTss,
	UnableToConnectRpc,
}

struct TaskRecord {
	timestamp: Instant,
	total_tasks: u64,
	tasks_failed: u64,
}

#[derive(Clone, Default)]
struct State {
	config: Arc<Mutex<Option<Config>>>,
	shard: Arc<Mutex<Option<ShardId>>>,
	last_block_ping: Arc<Mutex<Option<Instant>>>,
	task_records: Arc<Mutex<Vec<TaskRecord>>>,
}

impl State {
	async fn apply(&self, msg: AdminMsg) {
		match msg {
			AdminMsg::SetConfig(config) => {
				let mut gconfig = self.config.lock().await;
				*gconfig = Some(config);
			},
			AdminMsg::JoinedShard(shard_id) => {
				let mut working_shard = self.shard.lock().await;
				*working_shard = Some(shard_id);
			},
			AdminMsg::TargetBlockReceived => {
				let mut block_ping = self.last_block_ping.lock().await;
				*block_ping = Some(Instant::now());
			},
			AdminMsg::FailedTasks(total_tasks, tasks_failed) => {
				let mut task_records = self.task_records.lock().await;
				let now = Instant::now();
				task_records.retain(|record| {
					// delete all 5 mins old tasks
					now.duration_since(record.timestamp) <= Duration::from_secs(300)
				});
				task_records.push(TaskRecord {
					timestamp: now,
					total_tasks,
					tasks_failed,
				});
			},
		}
	}

	async fn get_tasks_failed_status(&self) -> (u64, u64) {
		let task_records = self.task_records.lock().await;

		let mut total_tasks = 0;
		let mut failed_tasks = 0;

		// Sum total tasks and failed tasks from the records within the last 5 minutes
		for record in task_records.iter() {
			total_tasks += record.total_tasks;
			failed_tasks += record.tasks_failed;
		}

		(total_tasks, failed_tasks)
	}
}

pub async fn listen(port: u16, mut admin: mpsc::Receiver<AdminMsg>) -> Result<()> {
	let state = State::default();
	let mut app = tide::with_state(state.clone());
	app.at("/config").get(config);
	app.at("/shard_id").get(shard_id);
	app.at("/health").get(health);
	let mut listen = app.listen(format!("0.0.0.0:{}", port)).boxed();
	loop {
		futures::select! {
			r = (&mut listen).fuse() => r?,
			msg = admin.next() => {
				if let Some(msg) = msg {
					state.apply(msg).await;
				}
			}
		}
	}
}

// `/config`
async fn config(req: Request<State>) -> tide::Result {
	let config = req.state().config.lock().await;
	let (code, body) = if let Some(config) = &*config {
		(StatusCode::Ok, Body::from_json(&config)?)
	} else {
		(StatusCode::ServiceUnavailable, Body::empty())
	};
	let mut r = Response::new(code);
	r.set_body(body);
	Ok(r)
}

// `/shard_id`
async fn shard_id(req: Request<State>) -> tide::Result {
	let shard_id = req.state().shard.lock().await;
	let (code, body) = if let Some(shard_id) = &*shard_id {
		(StatusCode::Ok, Body::from_json(&shard_id)?)
	} else {
		(StatusCode::ServiceUnavailable, Body::empty())
	};
	let mut r = Response::new(code);
	r.set_body(body);
	Ok(r)
}

// GET `/health`
async fn health(req: Request<State>) -> tide::Result {
	let block_ping = req.state().last_block_ping.lock().await;
	let ping_time =
		if let Some(ping_instant) = &*block_ping { ping_instant.elapsed().as_secs() } else { 0 };

	let (total_tasks, failed_tasks) = req.state().get_tasks_failed_status().await;
	let failed_percentage =
		if total_tasks > 0 { (failed_tasks as f64 / total_tasks as f64) * 100.0 } else { 0.0 };

	let response_body = json!({
		"external_node_ping_in_secs": ping_time,
		"failed_tasks_percentage": failed_percentage,
	});

	let (code, body) = if ping_time > 300 || failed_percentage > 50.0 {
		(StatusCode::InternalServerError, Body::from_json(&response_body)?)
	} else {
		(StatusCode::Ok, Body::from_json(&response_body)?)
	};
	let mut r = Response::new(code);
	r.set_body(body);
	Ok(r)
}
