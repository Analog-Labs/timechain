use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use serde::Serialize;
use std::future::IntoFuture;
use std::sync::Arc;
use time_primitives::admin::Config;
use time_primitives::ShardId;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum AdminMsg {
	SetConfig(Config),
	SetShards(Vec<ShardId>),
	NewBlock(u64),
	NewTargetBlock(u64),
	NewGasPrice(u128),
}

#[derive(Default)]
struct InnerState {
	shards: Vec<ShardId>,
	blocks: Blocks,
	gas_price: u128,
}

#[derive(Default, Serialize)]
struct Blocks {
	timechain_block: u64,
	target_block: u64,
}

#[derive(Clone, Default)]
struct AppState {
	config: Arc<Mutex<Option<Config>>>,
	inner: Arc<Mutex<InnerState>>,
}

impl AppState {
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
			AdminMsg::NewGasPrice(gas_price) => {
				let mut inner = self.inner.lock().await;
				inner.gas_price = gas_price;
			},
		}
	}
}

pub async fn listen(port: u16, mut admin: mpsc::Receiver<AdminMsg>) -> Result<()> {
	let state = AppState::default();

	let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
	tracing::info!("Loading admin interface: {}", addr);

	let app = axum::routing::Router::new()
		.route("/config", axum::routing::get(config))
		.route("/shards", axum::routing::get(shards))
		.route("/blocks", axum::routing::get(blocks))
		.route("/gas_price", axum::routing::get(gas_price))
		.with_state(state.clone());

	let mut listen = axum::serve(TcpListener::bind(&addr).await?, app).into_future();
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
async fn config(State(state): State<AppState>) -> Response {
	let config = state.config.lock().await;
	if let Some(config) = &*config {
		axum::Json(&config).into_response()
	} else {
		StatusCode::SERVICE_UNAVAILABLE.into_response()
	}
}

// `/shards`
async fn shards(State(state): State<AppState>) -> Response {
	let inner = state.inner.lock().await;
	axum::Json(&inner.shards).into_response()
}

// GET `/blocks`
async fn blocks(State(state): State<AppState>) -> Response {
	let inner = state.inner.lock().await;
	axum::Json(&inner.blocks).into_response()
}

// GET `/gas_price`
async fn gas_price(State(state): State<AppState>) -> Response {
	let inner = state.inner.lock().await;
	axum::Json(&inner.gas_price).into_response()
}
