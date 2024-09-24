use anyhow::Result;
use serde::{Deserialize, Serialize};
use tide::Body;
use tide::Request;
use time_primitives::NetworkId;

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
	network: NetworkId,
	account: String,
	address: String,
	peer_id: String,
}

impl Config {
	pub fn new(network: NetworkId, account: String, address: String, peer_id: String) -> Self {
		Self {
			network,
			account,
			address,
			peer_id,
		}
	}
}

pub async fn start(port: u16, config: Config) -> Result<()> {
	let mut app = tide::with_state(config);
	app.at("/config")
		.get(move |req: Request<Keys>| async move { Body::from_json(&req.state()) });
	app.listen(format!("0.0.0.0:{}", port)).await?;
	Ok(())
}
