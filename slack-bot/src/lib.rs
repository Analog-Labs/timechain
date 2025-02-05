mod github;
mod slack;

pub use crate::github::{Command, GithubState};
pub use crate::slack::SlackState;
use anyhow::Result;
use slack_morphism::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Env {
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

impl std::str::FromStr for Env {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		Ok(match s {
			"development" => Self::Development,
			"integration" => Self::Integration,
			"testnet" => Self::Testnet,
			"mainnet" => Self::Mainnet,
			_ => anyhow::bail!("unknown env"),
		})
	}
}

impl Env {
	pub fn from_channel(channel: &SlackChannelId) -> Option<Self> {
		Some(match channel.0.as_str() {
			"C08A621SKRR" => Env::Development,
			"C08BK21RPHA" => Env::Integration,
			"C08B20E4NEB" => Env::Testnet,
			"C08BV777PG9" => Env::Mainnet,
			_ => return None,
		})
	}
}
