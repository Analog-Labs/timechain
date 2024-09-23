use anyhow::Result;
use serde::{Deserialize, Serialize};
use tide::Body;
use tide::Request;

#[derive(Clone, Deserialize, Serialize)]
pub struct Keys {
	timechain: String,
	target: String,
}

impl Keys {
	pub fn new(timechain: String, target: String) -> Self {
		Self { timechain, target }
	}
}

pub async fn start(port: u16, keys: Keys) -> Result<()> {
	let mut app = tide::with_state(keys);
	app.at("/keys")
		.get(move |req: Request<Keys>| async move { Body::from_json(&req.state()) });
	app.listen(format!("0.0.0.0:{}", port)).await?;
	Ok(())
}
