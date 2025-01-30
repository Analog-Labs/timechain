use anyhow::Result;
use slack_morphism::prelude::*;

#[derive(Default)]
pub struct Sender {
	slack: Option<Slack>,
}

impl Sender {
	pub fn new() -> Self {
		Self { slack: Slack::new().ok() }
	}

	pub async fn text(&self, text: String) -> Result<()> {
		println!("{text}");
		if let Some(slack) = self.slack.as_ref() {
			slack.post_message(text).await?;
		}
		Ok(())
	}

	pub async fn csv(&self, title: &str, csv: Vec<u8>) -> Result<()> {
		println!("{}", csv_to_table::from_reader(&mut &csv[..])?);
		if let Some(slack) = self.slack.as_ref() {
			slack.upload_file("text/csv".into(), format!("{title}.csv"), csv).await?;
		}
		Ok(())
	}

	pub async fn log(&self, logs: Vec<String>) -> Result<()> {
		let logs: String = logs.join("\n");
		println!("{logs}");
		if let Some(slack) = self.slack.as_ref() {
			slack.post_message(format!("```{logs}```")).await?;
		}
		Ok(())
	}
}

struct Slack {
	client: SlackHyperClient,
	token: SlackApiToken,
	channel: SlackChannelId,
}

impl Slack {
	pub fn new() -> Result<Self> {
		let token = std::env::var("SLACK_BOT_TOKEN")?;
		let token_value = SlackApiTokenValue::new(token);
		let token = SlackApiToken::new(token_value);
		let channel = std::env::var("SLACK_CHANNEL_ID")?;
		let channel = SlackChannelId::new(channel);
		rustls::crypto::ring::default_provider()
			.install_default()
			.expect("Failed to install rustls crypto provider");
		let client = SlackClient::new(SlackClientHyperConnector::new()?);
		Ok(Self { client, token, channel })
	}

	pub async fn post_message(&self, message: String) -> Result<()> {
		let session = self.client.open_session(&self.token);
		let req = SlackApiChatPostMessageRequest::new(
			self.channel.clone(),
			SlackMessageContent::new().with_text(message),
		);
		session.chat_post_message(&req).await?;
		Ok(())
	}

	pub async fn upload_file(&self, mime: String, name: String, content: Vec<u8>) -> Result<()> {
		let session = self.client.open_session(&self.token);

		let req = SlackApiFilesGetUploadUrlExternalRequest::new(name, content.len());
		let resp = session.get_upload_url_external(&req).await?;

		let req = SlackApiFilesUploadViaUrlRequest::new(resp.upload_url, content, mime);
		session.files_upload_via_url(&req).await?;

		let req =
			SlackApiFilesCompleteUploadExternalRequest::new(vec![SlackApiFilesComplete::new(
				resp.file_id,
			)])
			.with_channel_id(self.channel.clone());
		session.files_complete_upload_external(&req).await?;
		Ok(())
	}
}
