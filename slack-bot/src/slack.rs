use crate::CommandInfo;
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
		command: SlackCommandId,
		text: String,
	) -> Result<SlackTs> {
		let session = self.client.open_session(&self.token);
		let req = SlackApiChatPostMessageRequest::new(
			channel,
			SlackMessageContent::new().with_text(format!("{} {}", command.0, text).trim().into()),
		);
		let resp = session.chat_post_message(&req).await?;
		Ok(resp.ts)
	}

	pub async fn update_command_info(&self, info: CommandInfo) -> Result<()> {
		let session = self.client.open_session(&self.token);
		let channel = info.env.channel();
		let content = SlackMessageContent::new()
			.with_text(info.command.to_string())
			.with_attachments(vec![SlackMessageAttachment::new()
				.with_color(info.color.to_string())
				.with_blocks(vec![SlackBlock::Section(SlackSectionBlock::new().with_fields(
					vec![
						SlackBlockMarkDownText::new(format!(
							"*Status*\n{}\n\n*Workflow*\n<{}|{} #{}>",
							info.status,
							info.url,
							info.command.short(),
							info.run_attempt,
						))
						.into(),
						SlackBlockMarkDownText::new(format!(
							"*Commit*\n`{} ({})`{}",
							&info.commit[..8],
							info.command.branch(),
							info.command.tag()
							.map(|tag| format!("\n\n*Version*\n`{}`", tag)).unwrap_or_default()
						))
						.into(),
					],
				))])]);
		let req = SlackApiChatUpdateRequest::new(channel, content, info.thread);
		session.chat_update(&req).await?;
		Ok(())
	}
}
