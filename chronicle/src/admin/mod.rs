use anyhow::Result;
use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tide::{Body, Request, Response, StatusCode};
use time_primitives::admin::Config;
use time_primitives::ShardId;
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum AdminMsg {
	SetConfig(Config),
	SetShards(Vec<ShardId>),
	NewBlock(u64),
	NewTargetBlock(u64),
}

#[derive(Default)]
struct InnerState {
	shards: Vec<ShardId>,
	blocks: Blocks,
}

#[derive(Default, Serialize)]
struct Blocks {
	timechain_block: u64,
	target_block: u64,
}

#[derive(Clone, Default)]
struct State {
	config: Arc<Mutex<Option<Config>>>,
	inner: Arc<Mutex<InnerState>>,
}

impl State {
	async fn apply(&self, msg: AdminMsg) {
		match msg {
			AdminMsg::SetConfig(config) => {
				let mut gconfig = self.config.lock().await;
				*gconfig = Some(config);
			},
			AdminMsg::SetShards(shards) => {
				let mut inner = self.inner.lock().await;
				inner.shards = shards;
			},
			AdminMsg::NewBlock(timechain_block) => {
				let mut inner = self.inner.lock().await;
				inner.blocks.timechain_block = timechain_block;
			},
			AdminMsg::NewTargetBlock(target_block) => {
				let mut inner = self.inner.lock().await;
				inner.blocks.target_block = target_block;
			},
		}
	}
}

pub async fn listen(port: u16, mut admin: mpsc::Receiver<AdminMsg>) -> Result<()> {
	let state = State::default();
	let mut app = tide::with_state(state.clone());
	app.at("/config").get(config);
	app.at("/shards").get(shards);
	app.at("/blocks").get(blocks);
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

// `/shards`
async fn shards(req: Request<State>) -> tide::Result {
	let inner = req.state().inner.lock().await;
	let mut r = Response::new(StatusCode::Ok);
	r.set_body(Body::from_json(&inner.shards)?);
	Ok(r)
}

// GET `/blocks`
async fn blocks(req: Request<State>) -> tide::Result {
	let inner = req.state().inner.lock().await;
	let mut r = Response::new(StatusCode::Ok);
	r.set_body(Body::from_json(&inner.blocks)?);
	Ok(r)
}
