use anyhow::Result;
use serde::{Deserialize, Serialize};
use tide::Body;
use tide::Request;
use time_primitives::NetworkId;

#[derive(Clone, Deserialize, Serialize)]
pub struct Keys {
	network: NetworkId,
	timechain: String,
	target: String,
}

impl Keys {
	pub fn new(network: NetworkId, timechain: String, target: String) -> Self {
		Self { network, timechain, target }
	}
}

pub async fn start(port: u16, keys: Keys) -> Result<()> {
	let mut app = tide::with_state(keys);
	app.at("/keys")
		.get(move |req: Request<Keys>| async move { Body::from_json(&req.state()) });
	app.listen(format!("0.0.0.0:{}", port)).await?;
	Ok(())
}
