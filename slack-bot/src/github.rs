use crate::Env;
use anyhow::{Context, Result};
use reqwest::Client;
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
			Self::TcCli { branch, tag, args } => {
				write!(f, "/tc-cli branch={branch} tag={tag} {args}")
			},
			Self::RuntimeUpgrade { branch } => write!(f, "/runtime-upgrade branch={branch}"),
			Self::DeployChronicles { branch, tag } => {
				write!(f, "/deploy-chronicles branch={branch} tag={tag}")
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

	fn branch(&self) -> &str {
		match self {
			Self::TcCli { branch, .. } => branch,
			Self::RuntimeUpgrade { branch } => branch,
			Self::DeployChronicles { branch, .. } => branch,
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

	fn inputs(&self, env: Env, thread: SlackTs) -> serde_json::Value {
		match self {
			Self::TcCli { tag, args, .. } => {
				json!({
					"environment": env.to_string(),
					"version": tag,
					"args": args,
					"thread": thread.0,
				})
			},
			Self::RuntimeUpgrade { .. } => {
				json!({
					"environment": env.to_string(),
					"thread": thread.0,
				})
			},
			Self::DeployChronicles { tag, .. } => {
				json!({
					"environment": env.to_string(),
					"version": tag,
					"thread": thread.0,
				})
			},
		}
	}

	fn json(&self, env: Env, thread: SlackTs) -> serde_json::Value {
		let inputs = self.inputs(env, thread);
		serde_json::json!({
			"ref": self.branch(),
			"inputs": inputs,
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

	pub async fn trigger_workflow(&self, command: &Command, env: Env, ts: SlackTs) -> Result<()> {
		tracing::info!("triggering {command} in {env}");
		let request = self
			.client
			.post(command.url())
			.json(&command.json(env, ts))
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
