use anyhow::Result;
use axum::Extension;
use slack_morphism::prelude::*;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
	let subscriber = tracing_subscriber::fmt()
		.with_env_filter("axum_events_api_server=debug,slack_morphism=debug")
		.finish();
	tracing::subscriber::set_global_default(subscriber)?;
	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("Failed to install rustls crypto provider");
	server().await?;
	Ok(())
}

fn error_handler(
	err: Box<dyn std::error::Error + Send + Sync>,
	_client: Arc<SlackHyperClient>,
	_states: SlackClientEventsUserState,
) -> HttpStatusCode {
	tracing::error!("{:#?}", err);

	// Defines what we return Slack server
	HttpStatusCode::BAD_REQUEST
}

async fn oauth_install_function(
	resp: SlackOAuthV2AccessTokenResponse,
	_client: Arc<SlackHyperClient>,
	_states: SlackClientEventsUserState,
) {
	tracing::info!("{:#?}", resp);
}

async fn command_event(
	Extension(_environment): Extension<Arc<SlackHyperListenerEnvironment>>,
	Extension(event): Extension<SlackCommandEvent>,
) -> axum::Json<SlackCommandEventResponse> {
	tracing::info!("Received command event: {:?}", event);
	axum::Json(SlackCommandEventResponse::new(
		SlackMessageContent::new().with_text("Working on it".into()),
	))
}

async fn server() -> Result<()> {
	let client: Arc<SlackHyperClient> =
		Arc::new(SlackClient::new(SlackClientHyperConnector::new()?));

	let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
	tracing::info!("Loading server: {}", addr);

	let oauth_listener_config = SlackOAuthListenerConfig::new(
		std::env::var("SLACK_CLIENT_ID")?.into(),
		std::env::var("SLACK_CLIENT_SECRET")?.into(),
		std::env::var("SLACK_BOT_SCOPE")?,
		std::env::var("SLACK_REDIRECT_HOST")?,
	);

	let listener_environment: Arc<SlackHyperListenerEnvironment> = Arc::new(
		SlackClientEventsListenerEnvironment::new(client.clone()).with_error_handler(error_handler),
	);
	let signing_secret: SlackSigningSecret = std::env::var("SLACK_SIGNING_SECRET")?.into();

	let listener: SlackEventsAxumListener<SlackHyperHttpsConnector> =
		SlackEventsAxumListener::new(listener_environment.clone());

	// build our application route with OAuth nested router and Push/Command/Interaction events
	let app = axum::routing::Router::new()
		.nest(
			"/auth",
			listener.oauth_router("/auth", &oauth_listener_config, oauth_install_function),
		)
		.route(
			"/command",
			axum::routing::post(command_event).layer(
				listener
					.events_layer(&signing_secret)
					.with_event_extractor(SlackEventsExtractors::command_event()),
			),
		);

	axum::serve(TcpListener::bind(&addr).await.unwrap(), app).await.unwrap();

	Ok(())
}
