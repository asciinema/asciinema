use anyhow::Result;

use crate::api;
use crate::asciicast;
use crate::cli;
use crate::config::Config;

impl cli::Upload {
    pub fn run(self) -> Result<()> {
        let config = Config::new(self.server_url.clone())?;
        let _ = asciicast::open_from_path(&self.file)?;
        let response = api::upload_asciicast(&self.file, &config)?;
        println!("{}", response.message.unwrap_or(response.url));

        Ok(())
    }
}
