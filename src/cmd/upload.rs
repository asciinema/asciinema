use super::Command;
use crate::api;
use crate::asciicast;
use crate::cli;
use crate::config::Config;
use anyhow::Result;

impl Command for cli::Upload {
    fn run(self, config: &Config) -> Result<()> {
        let _ = asciicast::open_from_path(&self.filename)?;
        let response = api::upload_asciicast(&self.filename, config)?;
        println!("{}", response.message.unwrap_or(response.url));

        Ok(())
    }
}
