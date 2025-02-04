use anyhow::Result;
use axum::Extension;
use reqwest::Client;
use serde_json::json;
use slack_morphism::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
	TcCli { tag: String, args: String },
	RuntimeUpgrade,
	DeployChronicles { tag: String },
}

impl std::fmt::Display for Command {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::TcCli { tag, args } => write!(f, "/tc-cli tag={tag} {args}"),
			Self::RuntimeUpgrade => write!(f, "/runtime-upgrade"),
			Self::DeployChronicles { tag } => write!(f, "/deploy-chronicles tag={tag}"),
		}
	}
}

impl Command {
	fn workflow_id(&self) -> &str {
		match self {
			Self::TcCli { .. } => "140077716",
			Self::RuntimeUpgrade { .. } => "105313512",
			Self::DeployChronicles { .. } => "125723565",
		}
	}

	fn url(&self) -> String {
		let owner = "analog-labs";
		let repo = "timechain";
		let workflow_id = self.workflow_id();
		format!("https://api.github.com/repos/{owner}/{repo}/actions/workflows/{workflow_id}/dispatches")
	}

	fn inputs(&self, env: Env, thread: SlackTs) -> serde_json::Value {
		match self {
			Self::TcCli { tag, args } => {
				json!({
					"environment": env.to_string(),
					"version": tag,
					"args": args,
					"thread": thread.0,
				})
			},
			Self::RuntimeUpgrade => {
				json!({
					"environment": env.to_string(),
					"thread": thread.0,
				})
			},
			Self::DeployChronicles { tag } => {
				json!({
					"environment": env.to_string(),
					"version": tag,
					"thread": thread.0,
				})
			},
		}
	}

	fn json(&self, branch: &str, env: Env, thread: SlackTs) -> serde_json::Value {
		let inputs = self.inputs(env, thread);
		serde_json::json!({
			"ref": branch,
			"inputs": inputs,
		})
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Env {
	Development,
	Integration,
	Testnet,
	Mainnet,
}

impl std::fmt::Display for Env {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Development => write!(f, "development"),
			Self::Integration => write!(f, "integration"),
			Self::Testnet => write!(f, "testnet"),
			Self::Mainnet => write!(f, "mainnet"),
		}
	}
}

#[derive(Clone)]
struct GithubState {
	client: Client,
	token: String,
	user_agent: String,
}

impl GithubState {
	fn new() -> Result<Self> {
		let client = Client::new();
		let token = std::env::var("GITHUB_TOKEN")?;
		let user_agent = std::env::var("GITHUB_USER_AGENT")?;
		Ok(Self { client, token, user_agent })
	}

	async fn trigger_workflow(
		&self,
		command: &Command,
		branch: &str,
		env: Env,
		ts: SlackTs,
	) -> Result<()> {
		tracing::info!("triggering {command} in {env}");
		let request = self
			.client
			.post(command.url())
			.json(&command.json(branch, env, ts))
			.bearer_auth(&self.token)
			.header("Accept", "application/vnd.github+json")
			.header("X-Github-Api-Version", "2022-11-28")
			.header("User-Agent", &self.user_agent)
			.build()?;
		let resp = self.client.execute(request).await?;
		let status = resp.status();
		if !status.is_success() {
			let body = resp.text().await?;
			anyhow::bail!("request failed with status code {status} {body:?}");
		}
		Ok(())
	}
}

#[derive(Clone)]
struct SlackState {
	client: SlackHyperClient,
	token: SlackApiToken,
}

impl SlackState {
	fn new() -> Result<Self> {
		let token = std::env::var("SLACK_BOT_TOKEN")?;
		let token_value = SlackApiTokenValue::new(token);
		let token = SlackApiToken::new(token_value);
		let client = SlackClient::new(SlackClientHyperConnector::new()?);
		Ok(Self { client, token })
	}

	async fn post_command(&self, channel: SlackChannelId, command: &Command) -> Result<SlackTs> {
		let session = self.client.open_session(&self.token);
		let req = SlackApiChatPostMessageRequest::new(
			channel,
			SlackMessageContent::new().with_text(command.to_string()),
		);
		let resp = session.chat_post_message(&req).await?;
		Ok(resp.ts)
	}
}

fn slack_error(error: String) -> axum::Json<SlackCommandEventResponse> {
	tracing::error!("{error}");
	axum::Json(SlackCommandEventResponse::new(SlackMessageContent::new().with_text(error)))
}

async fn command_event(
	Extension(gh): Extension<GithubState>,
	Extension(slack): Extension<SlackState>,
	Extension(_environment): Extension<Arc<SlackHyperListenerEnvironment>>,
	Extension(event): Extension<SlackCommandEvent>,
) -> axum::Json<SlackCommandEventResponse> {
	tracing::info!("Received command event: {:?}", event);
	let text = event.text.unwrap_or_default();
	let kv: HashMap<&str, &str> = text.split(' ').filter_map(|kv| kv.split_once('=')).collect();
	let tag = kv.get("tag").copied().unwrap_or("latest");
	let branch = kv.get("branch").copied().unwrap_or("development");
	let args = text
		.split(' ')
		.filter(|s| !s.starts_with("tag="))
		.filter(|s| !s.starts_with("branch="))
		.collect::<Vec<_>>()
		.as_slice()
		.join(" ");
	let command = match event.command.0.as_str() {
		"/tc-cli" => Command::TcCli { tag: tag.into(), args },
		"/runtime-upgrade" => Command::RuntimeUpgrade,
		"/deploy-chronicles" => Command::DeployChronicles { tag: tag.into() },
		_ => {
			return slack_error(format!("unknown command {}", &event.command.0));
		},
	};
	let env = match event.channel_id.0.as_str() {
		"C08A621SKRR" => Env::Development,
		"C08BK21RPHA" => Env::Integration,
		"C08B20E4NEB" => Env::Testnet,
		"C08BV777PG9" => Env::Mainnet,
		_ => {
			return slack_error(format!("unknown channel {}", &event.channel_id.0));
		},
	};

	let ts = match slack.post_command(event.channel_id, &command).await {
		Ok(ts) => ts,
		Err(err) => return slack_error(format!("starting slack thread failed: {err}")),
	};
	if let Err(err) = gh.trigger_workflow(&command, branch, env, ts).await {
		return slack_error(format!("triggering workflow failed: {err}"));
	}
	axum::Json(SlackCommandEventResponse::new(SlackMessageContent::new()))
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
	let signing_secret: SlackSigningSecret = std::env::var("SLACK_SIGNING_SECRET")?.into();

	let listener: SlackEventsAxumListener<SlackHyperHttpsConnector> =
		SlackEventsAxumListener::new(listener_environment.clone());

	// build our application route with OAuth nested router and Push/Command/Interaction events
	let app = axum::routing::Router::new()
		.route(
			"/command",
			axum::routing::post(command_event).layer(
				listener
					.events_layer(&signing_secret)
					.with_event_extractor(SlackEventsExtractors::command_event()),
			),
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
