use anyhow::Result;
use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use std::sync::Arc;
use tide::{Body, Request, Response, StatusCode};
use time_primitives::admin::Config;
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum AdminMsg {
	SetConfig(Config),
}

#[derive(Clone, Default)]
struct State {
	config: Arc<Mutex<Option<Config>>>,
}

impl State {
	async fn apply(&self, msg: AdminMsg) {
		match msg {
			AdminMsg::SetConfig(config) => {
				let mut gconfig = self.config.lock().await;
				*gconfig = Some(config);
			},
		}
	}
}

pub async fn listen(port: u16, mut admin: mpsc::Receiver<AdminMsg>) -> Result<()> {
	let state = State::default();
	let mut app = tide::with_state(state.clone());
	app.at("/config").get(move |req: Request<State>| async move {
		let config = req.state().config.lock().await;
		let (code, body) = if let Some(config) = &*config {
			(StatusCode::Ok, Body::from_json(&config)?)
		} else {
			(StatusCode::ServiceUnavailable, Body::empty())
		};
		let mut r = Response::new(code);
		r.set_body(body);
		Ok(r)
	});
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
