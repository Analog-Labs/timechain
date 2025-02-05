use anyhow::{Context, Result};
use axum::extract::{FromRef, State};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum_github_webhook_extract::{GithubEvent, GithubToken};
use slack_bot::{Command, CommandInfo, Env, GithubState, SlackState};
use slack_morphism::prelude::*;
use std::sync::Arc;
use tokio::net::TcpListener;

fn slack_error(error: String) -> Response {
	tracing::error!("{error}");
	axum::Json(SlackCommandEventResponse::new(SlackMessageContent::new().with_text(error)))
		.into_response()
}

async fn slack_command_event(
	Extension(gh): Extension<GithubState>,
	Extension(slack): Extension<SlackState>,
	Extension(_environment): Extension<Arc<SlackHyperListenerEnvironment>>,
	Extension(event): Extension<SlackCommandEvent>,
) -> Response {
	tracing::info!("received slack command event: {:?}", event);
	let text = event.text.unwrap_or_default();
	let Some(command) = Command::parse(&event.command, &text) else {
		return slack_error(format!("unknown command {}", &event.command.0));
	};
	let Some(env) = Env::from_channel(&event.channel_id) else {
		return slack_error(format!("unknown channel {}", &event.channel_id.0));
	};
	let ts = match slack.post_command(event.channel_id, event.command, text).await {
		Ok(ts) => ts,
		Err(err) => return slack_error(format!("starting slack thread failed: {err}")),
	};
	if let Err(err) = gh.trigger_workflow(&command, env, ts).await {
		return slack_error(format!("triggering workflow failed: {err}"));
	}
	axum::body::Body::empty().into_response()
}

async fn github_webhook_event(
	State(state): State<AppState>,
	GithubEvent(event): GithubEvent<slack_bot::GithubEvent>,
) -> Response {
	tracing::info!("received github webhook event: {:?}", event);
	if let Some(info) = CommandInfo::parse(event) {
		tracing::info!("{info:?}");
		if let Err(err) = state.slack.update_command_info(info).await {
			tracing::error!("failed to update command info: {err}");
		}
	}
	axum::body::Body::empty().into_response()
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

#[derive(Clone, FromRef)]
struct AppState {
	slack: SlackState,
	token: GithubToken,
}

async fn server() -> Result<()> {
	let client: Arc<SlackHyperClient> =
		Arc::new(SlackClient::new(SlackClientHyperConnector::new()?));

	let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
	tracing::info!("Loading server: {}", addr);

	let listener_environment: Arc<SlackHyperListenerEnvironment> = Arc::new(
		SlackClientEventsListenerEnvironment::new(client.clone()).with_error_handler(error_handler),
	);
	let signing_secret: SlackSigningSecret = std::env::var("SLACK_SIGNING_SECRET")
		.context("failed to read SLACK_SIGNING_SECRET")?
		.into();

	let listener: SlackEventsAxumListener<SlackHyperHttpsConnector> =
		SlackEventsAxumListener::new(listener_environment.clone());

	let github_webhook_secret =
		std::env::var("GITHUB_WEBHOOK_SECRET").context("failed to read GITHUB_WEBHOOK_SECRET")?;

	let github = GithubState::new()?;
	let slack = SlackState::new()?;

	// build our application route with OAuth nested router and Push/Command/Interaction events
	let app = axum::routing::Router::new()
		.route(
			"/command",
			axum::routing::post(slack_command_event).layer(
				listener
					.events_layer(&signing_secret)
					.with_event_extractor(SlackEventsExtractors::command_event()),
			),
		)
		.route(
			"/github",
			axum::routing::post(github_webhook_event).with_state(AppState {
				slack: slack.clone(),
				token: GithubToken(Arc::new(github_webhook_secret)),
			}),
		)
		.layer(Extension(github))
		.layer(Extension(slack));

	axum::serve(TcpListener::bind(&addr).await.unwrap(), app).await.unwrap();

	Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
	dotenv::dotenv().ok();
	let subscriber = tracing_subscriber::fmt()
		.with_env_filter("axum_events_api_server=debug,slack_morphism=debug,info")
		.finish();
	tracing::subscriber::set_global_default(subscriber)?;
	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("Failed to install rustls crypto provider");
	server().await?;
	Ok(())
}
