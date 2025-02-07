use crate::Env;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use slack_morphism::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
	TcCli { branch: String, tag: String, args: String },
	RuntimeUpgrade { branch: String },
	DeployChronicles { branch: String, tag: String },
}

impl std::fmt::Display for Command {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::TcCli { args, .. } => {
				write!(f, "/tc-cli {args}")
			},
			Self::RuntimeUpgrade { .. } => write!(f, "/runtime-upgrade"),
			Self::DeployChronicles { .. } => {
				write!(f, "/deploy-chronicles")
			},
		}
	}
}

impl Command {
	pub fn parse(command: &SlackCommandId, text: &str) -> Option<Self> {
		let kv: HashMap<&str, &str> = text.split(' ').filter_map(|kv| kv.split_once('=')).collect();
		let tag = kv.get("tag").copied().unwrap_or("latest");
		let branch = kv.get("branch").copied().unwrap_or("development");
		let args = text
			.split(' ')
			.filter(|s| !s.starts_with("tag="))
			.filter(|s| !s.starts_with("branch="))
			.collect::<Vec<_>>()
			.as_slice()
			.join(" ")
			.trim()
			.to_string();
		Some(match command.0.as_str() {
			"/tc-cli" => Command::TcCli {
				branch: branch.into(),
				tag: tag.into(),
				args,
			},
			"/runtime-upgrade" => Command::RuntimeUpgrade { branch: branch.into() },
			"/deploy-chronicles" => Command::DeployChronicles {
				branch: branch.into(),
				tag: tag.into(),
			},
			_ => return None,
		})
	}

	pub fn short(&self) -> &str {
		match self {
			Self::TcCli { .. } => "tc-cli",
			Self::RuntimeUpgrade { .. } => "upgrade-runtime",
			Self::DeployChronicles { .. } => "deploy-chronicles",
		}
	}

	pub fn branch(&self) -> &str {
		match self {
			Self::TcCli { branch, .. } => branch,
			Self::RuntimeUpgrade { branch } => branch,
			Self::DeployChronicles { branch, .. } => branch,
		}
	}

	pub fn tag(&self) -> Option<&str> {
		match self {
			Self::TcCli { tag, .. } => Some(tag),
			Self::RuntimeUpgrade { .. } => None,
			Self::DeployChronicles { tag, .. } => Some(tag),
		}
	}

	pub fn set_branch(&mut self, b: String) {
		match self {
			Self::TcCli { branch, .. } => *branch = b,
			Self::RuntimeUpgrade { branch } => *branch = b,
			Self::DeployChronicles { branch, .. } => *branch = b,
		}
	}

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

	fn inputs(&self, env: Env, thread: SlackTs, user: String) -> serde_json::Value {
		match self {
			Self::TcCli { tag, args, .. } => {
				json!({
					"environment": env.to_string(),
					"version": tag,
					"args": args,
					"thread": thread.0,
					"user": user,
				})
			},
			Self::RuntimeUpgrade { .. } => {
				json!({
					"environment": env.to_string(),
					"thread": thread.0,
					"user": user,
				})
			},
			Self::DeployChronicles { tag, .. } => {
				json!({
					"environment": env.to_string(),
					"version": tag,
					"thread": thread.0,
					"user": user,
				})
			},
		}
	}

	fn json(&self, env: Env, thread: SlackTs, user: String) -> serde_json::Value {
		let inputs = self.inputs(env, thread, user);
		serde_json::json!({
			"ref": self.branch(),
			"inputs": inputs,
		})
	}
}

#[derive(Clone, Debug, Deserialize)]
pub struct GithubEvent {
	workflow_job: WorkflowJob,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WorkflowJob {
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

impl std::fmt::Display for Color {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Black => write!(f, "#6c757d"),
			Self::Green => write!(f, "#28a745"),
			Self::Red => write!(f, "#dc3545"),
		}
	}
}

#[derive(Clone, Debug)]
pub struct CommandInfo {
	pub thread: SlackTs,
	pub command: Command,
	pub env: Env,
	pub user: String,
	pub run_attempt: u32,
	pub commit: String,
	pub status: String,
	pub color: Color,
	pub url: String,
}

impl CommandInfo {
	pub fn parse(event: GithubEvent) -> Option<Self> {
		let job = event.workflow_job;
		let cmd = job.name.split_once("Run")?.1.trim();
		let (cmd, rest) = cmd.split_once(" on ")?;
		let (cmd, text) = cmd.split_once(' ')?;
		let (env, rest) = rest.split_once('#')?;
		let (thread, user) = rest.split_once('@')?;
		let mut command = Command::parse(&SlackCommandId(cmd.into()), text)?;
		let env = env.trim().parse().ok()?;
		let thread = SlackTs(thread.trim().into());
		let user = user.trim().into();
		command.set_branch(job.head_branch);
		Some(Self {
			thread,
			command,
			env,
			user,
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

#[derive(Clone)]
pub struct GithubState {
	client: Client,
	token: String,
	user_agent: String,
}

impl GithubState {
	pub fn new() -> Result<Self> {
		let client = Client::new();
		let token = std::env::var("GITHUB_TOKEN").context("failed to read GITHUB_TOKEN")?;
		let user_agent =
			std::env::var("GITHUB_USER_AGENT").context("failed to read GITHUB_USER_AGENT")?;
		Ok(Self { client, token, user_agent })
	}

	pub async fn trigger_workflow(
		&self,
		command: &Command,
		env: Env,
		ts: SlackTs,
		user: String,
	) -> Result<()> {
		tracing::info!("triggering {command} in {env}");
		let request = self
			.client
			.post(command.url())
			.json(&command.json(env, ts, user))
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
