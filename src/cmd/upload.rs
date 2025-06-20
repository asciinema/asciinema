use anyhow::Result;
use tokio::runtime::Runtime;

use crate::api;
use crate::asciicast;
use crate::cli;
use crate::config::Config;

impl cli::Upload {
    pub fn run(self) -> Result<()> {
        Runtime::new()?.block_on(self.do_run())
    }

    async fn do_run(self) -> Result<()> {
        let config = Config::new(self.server_url.clone())?;
        let _ = asciicast::open_from_path(&self.file)?;
        let response = api::upload_asciicast(&self.file, &config).await?;
        println!("{}", response.message.unwrap_or(response.url));

        Ok(())
    }
}
