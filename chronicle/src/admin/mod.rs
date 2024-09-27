use tide::Body;
use tide::Request;
use time_primitives::admin::Config;

pub fn start(port: u16, config: Config) {
	let mut app = tide::with_state(config);
	app.at("/config")
		.get(move |req: Request<Config>| async move { Body::from_json(&req.state()) });
	tokio::task::spawn(async move {
		app.listen(format!("0.0.0.0:{}", port)).await.unwrap();
	});
}
