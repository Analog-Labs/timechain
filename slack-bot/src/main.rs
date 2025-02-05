use anyhow::{Context, Result};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum_github_webhook_extract::{GithubEvent, GithubToken};
use serde::Deserialize;
use slack_bot::{Command, Env, GithubState, SlackState};
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
	let Some(command) = Command::parse(&event.command, &event.text.unwrap_or_default()) else {
		return slack_error(format!("unknown command {}", &event.command.0));
	};
	let Some(env) = Env::from_channel(&event.channel_id) else {
		return slack_error(format!("unknown channel {}", &event.channel_id.0));
	};
	let ts = match slack.post_command(event.channel_id, &command).await {
		Ok(ts) => ts,
		Err(err) => return slack_error(format!("starting slack thread failed: {err}")),
	};
	if let Err(err) = gh.trigger_workflow(&command, env, ts).await {
		return slack_error(format!("triggering workflow failed: {err}"));
	}
	axum::body::Body::empty().into_response()
}

#[derive(Clone, Debug, Deserialize)]
struct Event {
	workflow_job: WorkflowJob,
}

#[derive(Clone, Debug, Deserialize)]
struct WorkflowJob {
	run_attempt: u32,
	name: String,
	status: String,
	conclusion: Option<String>,
	html_url: String,
	head_branch: String,
	head_sha: String,
}

#[derive(Clone, Copy, Debug)]
pub enum Color {
	Black,
	Green,
	Red,
}

#[derive(Clone, Debug)]
pub struct CommandInfo {
	pub thread: SlackTs,
	pub command: Command,
	pub env: Env,
	pub run_attempt: u32,
	pub commit: String,
	pub status: String,
	pub color: Color,
	pub url: String,
}

impl CommandInfo {
	fn parse(job: WorkflowJob) -> Option<Self> {
		let cmd = job.name.split_once("Run")?.1.trim();
		let (cmd, rest) = cmd.split_once("on")?;
		let (cmd, text) = cmd.split_once(' ')?;
		let (env, thread) = rest.split_once('#')?;
		let mut command = Command::parse(&SlackCommandId(cmd.into()), text)?;
		let env = env.trim().parse().ok()?;
		let thread = SlackTs(thread.trim().into());
		command.set_branch(job.head_branch);
		Some(Self {
			thread,
			command,
			env,
			run_attempt: job.run_attempt,
			commit: job.head_sha,
			color: job
				.conclusion
				.as_ref()
				.map(|c| if c == "success" { Color::Green } else { Color::Red })
				.unwrap_or(Color::Black),
			status: job.conclusion.unwrap_or(job.status),
			url: job.html_url,
		})
	}
}

async fn github_webhook_event(GithubEvent(event): GithubEvent<Event>) -> Response {
	tracing::info!("received github webhook event: {:?}", event);
	if let Some(info) = CommandInfo::parse(event.workflow_job) {
		tracing::info!("{info:?}");
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
			axum::routing::post(github_webhook_event)
				.with_state(GithubToken(Arc::new(github_webhook_secret))),
		)
		.layer(Extension(GithubState::new()?))
		.layer(Extension(SlackState::new()?));

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
