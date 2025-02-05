use crate::Command;
use anyhow::{Context, Result};
use slack_morphism::prelude::*;

#[derive(Clone)]
pub struct SlackState {
	client: SlackHyperClient,
	token: SlackApiToken,
}

impl SlackState {
	pub fn new() -> Result<Self> {
		let token = std::env::var("SLACK_BOT_TOKEN").context("failed to read SLACK_BOT_TOKEN")?;
		let token_value = SlackApiTokenValue::new(token);
		let token = SlackApiToken::new(token_value);
		let client = SlackClient::new(SlackClientHyperConnector::new()?);
		Ok(Self { client, token })
	}

	pub async fn post_command(
		&self,
		channel: SlackChannelId,
		command: &Command,
	) -> Result<SlackTs> {
		let session = self.client.open_session(&self.token);
		let req = SlackApiChatPostMessageRequest::new(
			channel,
			SlackMessageContent::new().with_text(command.to_string()),
		);
		let resp = session.chat_post_message(&req).await?;
		Ok(resp.ts)
	}
}
