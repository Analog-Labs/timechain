use anyhow::Result;
use slack_morphism::prelude::*;

#[derive(Default)]
pub struct Sender {
	slack: Option<Slack>,
}

impl Sender {
	pub fn new() -> Self {
		let slack = match Slack::new() {
			Ok(slack) => Some(slack),
			Err(err) => {
				log::info!("not logging to slack: {err}");
				None
			},
		};
		Self { slack }
	}

	pub async fn text(&self, text: String) -> Result<()> {
		println!("{text}");
		if let Some(slack) = self.slack.as_ref() {
			slack.post_message(None, SlackMessageContent::new().with_text(text)).await?;
		}
		Ok(())
	}

	pub async fn csv(&self, title: &str, csv: Vec<u8>) -> Result<()> {
		println!("{}", csv_to_table::from_reader(&mut &csv[..])?);
		if let Some(slack) = self.slack.as_ref() {
			slack.post_table(None, title.into(), csv).await?;
		}
		Ok(())
	}

	pub async fn log(&self, logs: Vec<String>) -> Result<()> {
		let logs: String = logs.join("\n");
		println!("{logs}");
		if let Some(slack) = self.slack.as_ref() {
			let text = SlackBlockMarkDownText::new(format!("```{logs}```"));
			slack
				.post_message(
					None,
					SlackMessageContent::new().with_blocks(vec![SlackBlock::Section(
						SlackSectionBlock::new().with_text(text.into()),
					)]),
				)
				.await?;
		}
		Ok(())
	}
}

struct Slack {
	client: SlackHyperClient,
	token: SlackApiToken,
	channel: SlackChannelId,
	thread: SlackTs,
}

impl Slack {
	pub fn new() -> Result<Self> {
		let token = std::env::var("SLACK_BOT_TOKEN")?;
		let token_value = SlackApiTokenValue::new(token);
		let token = SlackApiToken::new(token_value);
		let channel = std::env::var("SLACK_CHANNEL_ID")?;
		let channel = SlackChannelId::new(channel);
		let thread = std::env::var("SLACK_THREAD_TS")?;
		let thread = SlackTs::new(thread);
		rustls::crypto::ring::default_provider()
			.install_default()
			.expect("Failed to install rustls crypto provider");
		let client = SlackClient::new(SlackClientHyperConnector::new()?);
		Ok(Self { client, token, channel, thread })
	}

	pub async fn post_message(
		&self,
		id: Option<SlackTs>,
		msg: SlackMessageContent,
	) -> Result<SlackTs> {
		let session = self.client.open_session(&self.token);
		if let Some(id) = id {
			let req = SlackApiChatUpdateRequest::new(self.channel.clone(), msg, id.clone());
			session.chat_update(&req).await?;
			Ok(id)
		} else {
			let req = SlackApiChatPostMessageRequest::new(self.channel.clone(), msg)
				.with_thread_ts(self.thread.clone());
			let resp = session.chat_post_message(&req).await?;
			Ok(resp.ts)
		}
	}

	pub async fn post_table(
		&self,
		id: Option<SlackFileId>,
		name: String,
		content: Vec<u8>,
	) -> Result<SlackFileId> {
		let session = self.client.open_session(&self.token);

		let req = SlackApiFilesGetUploadUrlExternalRequest::new(name, content.len());
		let resp = session.get_upload_url_external(&req).await?;

		let req =
			SlackApiFilesUploadViaUrlRequest::new(resp.upload_url, content, "text/csv".into());
		session.files_upload_via_url(&req).await?;

		if let Some(id) = id {
			let req = SlackApiFilesDeleteRequest::new(id);
			session.files_delete(&req).await?;
		}
		let req =
			SlackApiFilesCompleteUploadExternalRequest::new(vec![SlackApiFilesComplete::new(
				resp.file_id,
			)])
			.with_channel_id(self.channel.clone())
			.with_thread_ts(self.thread.clone());
		let resp = session.files_complete_upload_external(&req).await?;
		Ok(resp.files[0].id.clone())
	}
}
