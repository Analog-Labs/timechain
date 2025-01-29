use anyhow::Result;

pub struct Sender {}

impl Sender {
	pub fn new() -> Self {
		Self {}
	}

	pub async fn text(&self, text: &str) -> Result<()> {
		println!("{text}");
		Ok(())
	}

	pub async fn csv(&self, mut csv: &[u8]) -> Result<()> {
		println!("{}", csv_to_table::from_reader(&mut csv)?);
		Ok(())
	}

	pub async fn log(&self, logs: &[String]) -> Result<()> {
		for log in logs {
			println!("{log}");
		}
		Ok(())
	}
}
