use crate::api;
use crate::asciicast;
use crate::config::Config;
use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct Cli {
    /// Filename/path of asciicast to upload
    filename: String,
}

impl Cli {
    pub fn run(self, config: &Config) -> Result<()> {
        let _ = asciicast::open_from_path(&self.filename)?;
        let response = api::upload_asciicast(&self.filename, config)?;
        println!("{}", response.message.unwrap_or(response.url));

        Ok(())
    }
}
