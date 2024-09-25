use anyhow::Result;
use tide::Body;
use tide::Request;
use time_primitives::admin::Config;

pub async fn start(port: u16, config: Config) -> Result<()> {
	let mut app = tide::with_state(config);
	app.at("/config")
		.get(move |req: Request<Config>| async move { Body::from_json(&req.state()) });
	app.listen(format!("0.0.0.0:{}", port)).await?;
	Ok(())
}
